use serde::{Deserialize, Serialize};

const DEFAULT_SAMPLE_RATE: u32 = 24_000;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AudioInput {
    pub mime_type: String,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct StreamedAudioInput {
    pub mime_type: String,
    pub chunks: Vec<Vec<u8>>,
}

impl AudioInput {
    pub fn from_pcm16(samples: &[i16]) -> Self {
        Self::from_pcm16_with_rate(samples, DEFAULT_SAMPLE_RATE)
    }

    pub fn from_pcm16_with_rate(samples: &[i16], sample_rate: u32) -> Self {
        Self {
            mime_type: "audio/wav".to_owned(),
            bytes: wav_bytes_from_pcm16(samples, sample_rate),
        }
    }

    pub fn from_pcm32(samples: &[f32]) -> Self {
        Self::from_pcm16(&normalize_pcm32(samples))
    }
}

impl StreamedAudioInput {
    pub fn from_pcm16_chunks(chunks: &[Vec<i16>]) -> Self {
        Self {
            mime_type: "audio/pcm".to_owned(),
            chunks: chunks
                .iter()
                .map(|chunk| pcm16_bytes(chunk.as_slice()))
                .collect(),
        }
    }

    pub fn from_pcm32_chunks(chunks: &[Vec<f32>]) -> Self {
        let normalized = chunks
            .iter()
            .map(|chunk| normalize_pcm32(chunk.as_slice()))
            .collect::<Vec<_>>();
        Self::from_pcm16_chunks(&normalized)
    }
}

fn normalize_pcm32(samples: &[f32]) -> Vec<i16> {
    samples
        .iter()
        .map(|sample| (sample.clamp(-1.0, 1.0) * 32_767.0) as i16)
        .collect()
}

fn pcm16_bytes(samples: &[i16]) -> Vec<u8> {
    samples
        .iter()
        .flat_map(|sample| sample.to_le_bytes())
        .collect()
}

fn wav_bytes_from_pcm16(samples: &[i16], sample_rate: u32) -> Vec<u8> {
    let data_bytes = pcm16_bytes(samples);
    let mut wav = Vec::with_capacity(44 + data_bytes.len());
    let byte_rate = sample_rate * 2;
    let block_align = 2u16;
    let chunk_size = 36 + data_bytes.len() as u32;

    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&chunk_size.to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&byte_rate.to_le_bytes());
    wav.extend_from_slice(&block_align.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&(data_bytes.len() as u32).to_le_bytes());
    wav.extend_from_slice(&data_bytes);
    wav
}

#[cfg(test)]
mod tests {
    use super::{AudioInput, StreamedAudioInput};

    #[test]
    fn buffered_audio_input_normalizes_float_samples_to_wav() {
        let input = AudioInput::from_pcm32(&[-1.5, -0.5, 0.0, 0.5, 1.5]);

        assert_eq!(input.mime_type, "audio/wav");
        assert!(input.bytes.starts_with(b"RIFF"));
        assert_eq!(&input.bytes[8..12], b"WAVE");
        assert_eq!(
            &input.bytes[input.bytes.len() - 10..],
            &[1, 128, 1, 192, 0, 0, 255, 63, 255, 127]
        );
    }

    #[test]
    fn streamed_audio_input_normalizes_chunks_to_pcm16_bytes() {
        let input = StreamedAudioInput::from_pcm32_chunks(&[vec![-1.0, 0.0], vec![0.5, 1.0, 2.0]]);

        assert_eq!(input.mime_type, "audio/pcm");
        assert_eq!(input.chunks.len(), 2);
        assert_eq!(input.chunks[0], vec![1, 128, 0, 0]);
        assert_eq!(input.chunks[1], vec![255, 63, 255, 127, 255, 127]);
    }
}
