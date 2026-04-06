use std::time::{SystemTime, UNIX_EPOCH};

use agents_core::{AgentsError, InputItem, Result, Session, SessionSettings};
use async_trait::async_trait;
use chacha20poly1305::aead::Aead;
use chacha20poly1305::{KeyInit, XChaCha20Poly1305, XNonce};
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Envelope stored in the underlying session for encrypted items.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncryptedEnvelope {
    pub version: u8,
    pub nonce_hex: String,
    pub ciphertext_hex: String,
    pub created_at_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct LegacyEncryptedEnvelope {
    pub nonce: u64,
    pub ciphertext_hex: String,
    pub created_at_ms: u64,
}

/// Transparent encrypted wrapper for any session implementation.
#[derive(Clone, Debug)]
pub struct EncryptedSession<S> {
    pub inner: S,
    pub encryption_key: String,
    pub ttl_seconds: Option<u64>,
}

impl<S> EncryptedSession<S> {
    pub fn new(inner: S, encryption_key: impl Into<String>) -> Self {
        Self {
            inner,
            encryption_key: encryption_key.into(),
            ttl_seconds: None,
        }
    }

    pub fn with_ttl_seconds(mut self, ttl_seconds: u64) -> Self {
        self.ttl_seconds = Some(ttl_seconds);
        self
    }
}

#[async_trait]
impl<S> Session for EncryptedSession<S>
where
    S: Session + Send + Sync,
{
    fn session_id(&self) -> &str {
        self.inner.session_id()
    }

    fn session_settings(&self) -> Option<&SessionSettings> {
        self.inner.session_settings()
    }

    async fn get_items_with_limit(&self, limit: Option<usize>) -> Result<Vec<InputItem>> {
        let items = self.inner.get_items_with_limit(limit).await?;
        let mut decrypted = Vec::new();
        for item in items {
            if let Some(value) = self.try_decrypt_item(item)? {
                decrypted.push(value);
            }
        }
        Ok(decrypted)
    }

    async fn add_items(&self, items: Vec<InputItem>) -> Result<()> {
        let encrypted = items
            .into_iter()
            .enumerate()
            .map(|(index, item)| self.encrypt_item(item, index as u64))
            .collect::<Result<Vec<_>>>()?;
        self.inner.add_items(encrypted).await
    }

    async fn pop_item(&self) -> Result<Option<InputItem>> {
        loop {
            let Some(item) = self.inner.pop_item().await? else {
                return Ok(None);
            };
            if let Some(value) = self.try_decrypt_item(item)? {
                return Ok(Some(value));
            }
        }
    }

    async fn clear_session(&self) -> Result<()> {
        self.inner.clear_session().await
    }
}

impl<S> EncryptedSession<S>
where
    S: Session + Send + Sync,
{
    fn encrypt_item(&self, item: InputItem, nonce: u64) -> Result<InputItem> {
        let _ = nonce;
        let plaintext =
            serde_json::to_vec(&item).map_err(|error| AgentsError::message(error.to_string()))?;
        let cipher = build_cipher(&self.encryption_key, self.session_id());
        let mut nonce_bytes = [0u8; 24];
        OsRng.fill_bytes(&mut nonce_bytes);
        let ciphertext = cipher
            .encrypt(XNonce::from_slice(&nonce_bytes), plaintext.as_ref())
            .map_err(|error| AgentsError::message(error.to_string()))?;
        let envelope = EncryptedEnvelope {
            version: 2,
            nonce_hex: encode_hex(&nonce_bytes),
            ciphertext_hex: encode_hex(&ciphertext),
            created_at_ms: now_ms(),
        };
        Ok(InputItem::Json {
            value: serde_json::to_value(envelope)
                .map_err(|error| AgentsError::message(error.to_string()))?,
        })
    }

    fn try_decrypt_item(&self, item: InputItem) -> Result<Option<InputItem>> {
        let InputItem::Json { value } = item else {
            return Ok(Some(item));
        };
        if let Ok(envelope) = serde_json::from_value::<EncryptedEnvelope>(value.clone()) {
            if let Some(ttl_seconds) = self.ttl_seconds {
                let age_ms = now_ms().saturating_sub(envelope.created_at_ms);
                if age_ms > ttl_seconds.saturating_mul(1_000) {
                    return Ok(None);
                }
            }

            let ciphertext = decode_hex(&envelope.ciphertext_hex)?;
            let nonce_bytes = decode_hex(&envelope.nonce_hex)?;
            if nonce_bytes.len() != 24 {
                return Err(AgentsError::message("invalid xchacha20 nonce length"));
            }
            let cipher = build_cipher(&self.encryption_key, self.session_id());
            let plaintext = cipher
                .decrypt(XNonce::from_slice(&nonce_bytes), ciphertext.as_ref())
                .map_err(|error| AgentsError::message(error.to_string()))?;
            let item = serde_json::from_slice::<InputItem>(&plaintext)
                .map_err(|error| AgentsError::message(error.to_string()))?;
            return Ok(Some(item));
        }

        let legacy: LegacyEncryptedEnvelope = match serde_json::from_value(value.clone()) {
            Ok(envelope) => envelope,
            Err(_) => {
                return Ok(Some(InputItem::Json { value }));
            }
        };
        if let Some(ttl_seconds) = self.ttl_seconds {
            let age_ms = now_ms().saturating_sub(legacy.created_at_ms);
            if age_ms > ttl_seconds.saturating_mul(1_000) {
                return Ok(None);
            }
        }

        let ciphertext = decode_hex(&legacy.ciphertext_hex)?;
        let keystream = derive_legacy_keystream(
            &self.encryption_key,
            self.session_id(),
            legacy.nonce,
            ciphertext.len(),
        );
        let plaintext = ciphertext
            .iter()
            .zip(keystream.iter())
            .map(|(lhs, rhs)| lhs ^ rhs)
            .collect::<Vec<_>>();
        let item = serde_json::from_slice::<InputItem>(&plaintext)
            .map_err(|error| AgentsError::message(error.to_string()))?;
        Ok(Some(item))
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn build_cipher(secret: &str, session_id: &str) -> XChaCha20Poly1305 {
    let mut hasher = Sha256::new();
    hasher.update(secret.as_bytes());
    hasher.update(b":");
    hasher.update(session_id.as_bytes());
    let key_bytes = hasher.finalize();
    XChaCha20Poly1305::new_from_slice(&key_bytes).expect("sha256 output should be 32 bytes")
}

fn derive_legacy_keystream(secret: &str, session_id: &str, nonce: u64, len: usize) -> Vec<u8> {
    let mut stream = Vec::with_capacity(len);
    let mut counter = 0u64;
    while stream.len() < len {
        let block =
            fnv1a64(format!("{secret}:{session_id}:{nonce}:{counter}").as_bytes()).to_le_bytes();
        stream.extend(block);
        counter = counter.wrapping_add(1);
    }
    stream.truncate(len);
    stream
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn decode_hex(value: &str) -> Result<Vec<u8>> {
    if !value.len().is_multiple_of(2) {
        return Err(AgentsError::message("invalid hex payload"));
    }
    let mut bytes = Vec::with_capacity(value.len() / 2);
    let chars = value.as_bytes();
    for index in (0..chars.len()).step_by(2) {
        let high = decode_hex_nibble(chars[index])?;
        let low = decode_hex_nibble(chars[index + 1])?;
        bytes.push((high << 4) | low);
    }
    Ok(bytes)
}

fn decode_hex_nibble(value: u8) -> Result<u8> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        b'A'..=b'F' => Ok(value - b'A' + 10),
        _ => Err(AgentsError::message("invalid hex digit")),
    }
}

#[cfg(test)]
mod tests {
    use agents_core::MemorySession;
    use serde_json::json;

    use super::*;

    #[tokio::test]
    async fn round_trips_items_through_envelope() {
        let inner = MemorySession::new("session");
        let session = EncryptedSession::new(inner.clone(), "secret");
        session
            .add_items(vec![InputItem::from("hello")])
            .await
            .expect("encrypted item should save");

        let items = session.get_items().await.expect("items should load");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].as_text(), Some("hello"));

        let stored = inner.get_items().await.expect("inner items should load");
        let envelope = match &stored[0] {
            InputItem::Json { value } => value,
            other => panic!("expected encrypted json envelope, got {other:?}"),
        };
        assert_eq!(envelope["version"], 2);
        assert!(envelope.get("nonce_hex").is_some());
        assert!(envelope.get("ciphertext_hex").is_some());
        assert_ne!(envelope["ciphertext_hex"], json!("hello"));
    }

    #[tokio::test]
    async fn reads_legacy_envelopes_for_backward_compatibility() {
        let inner = MemorySession::new("session");
        let legacy_plaintext = serde_json::to_vec(&InputItem::from("hello legacy"))
            .expect("legacy plaintext should serialize");
        let keystream =
            derive_legacy_keystream("secret", inner.session_id(), 0, legacy_plaintext.len());
        let legacy_ciphertext = legacy_plaintext
            .iter()
            .zip(keystream.iter())
            .map(|(lhs, rhs)| lhs ^ rhs)
            .collect::<Vec<_>>();
        inner
            .add_items(vec![InputItem::Json {
                value: json!({
                    "nonce": 0,
                    "ciphertext_hex": encode_hex(&legacy_ciphertext),
                    "created_at_ms": now_ms(),
                }),
            }])
            .await
            .expect("legacy item should save");

        let session = EncryptedSession::new(inner, "secret");
        let items = session.get_items().await.expect("items should decrypt");
        assert_eq!(items, vec![InputItem::from("hello legacy")]);
    }
}
