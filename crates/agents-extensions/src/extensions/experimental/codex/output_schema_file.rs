use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use agents_core::{AgentsError, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Materialized temporary schema file for Codex structured-output runs.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputSchemaFile {
    pub schema_path: Option<PathBuf>,
    cleanup_dir: Option<PathBuf>,
}

impl OutputSchemaFile {
    pub fn cleanup(&self) {
        if let Some(path) = &self.cleanup_dir {
            let _ = fs::remove_dir_all(path);
        }
    }
}

impl Drop for OutputSchemaFile {
    fn drop(&mut self) {
        self.cleanup();
    }
}

pub fn create_output_schema_file(schema: Option<&Value>) -> Result<OutputSchemaFile> {
    let Some(schema) = schema else {
        return Ok(OutputSchemaFile::default());
    };
    let Some(_) = schema.as_object() else {
        return Err(AgentsError::message(
            "output_schema must be a plain JSON object",
        ));
    };

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("codex-output-schema-{unique}"));
    fs::create_dir_all(&dir).map_err(|error| AgentsError::message(error.to_string()))?;
    let schema_path = dir.join("schema.json");
    let payload = serde_json::to_vec_pretty(schema)
        .map_err(|error| AgentsError::message(error.to_string()))?;
    if let Err(error) = fs::write(&schema_path, payload) {
        let _ = fs::remove_dir_all(&dir);
        return Err(AgentsError::message(error.to_string()));
    }

    Ok(OutputSchemaFile {
        schema_path: Some(schema_path),
        cleanup_dir: Some(dir),
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn creates_schema_file() {
        let file = create_output_schema_file(Some(&json!({"type":"object"})))
            .expect("schema file should be created");
        assert!(file.schema_path.as_ref().is_some_and(|path| path.exists()));
        file.cleanup();
    }
}
