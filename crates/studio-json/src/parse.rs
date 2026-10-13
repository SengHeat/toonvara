use std::path::Path;

use serde::Deserialize;
use studio_core::{AssetTable, OutputSettings, Project, Scene, VideoSettings, SCHEMA_VERSION};

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors returned when parsing a Toonvara video JSON file.
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    /// The file could not be read from disk.
    #[error("cannot read file: {0}")]
    Io(#[from] std::io::Error),

    /// The file contains invalid JSON or a missing/wrong-typed field.
    #[error("invalid JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),

    /// The `schema_version` field is present but has an unsupported value.
    #[error("unsupported schema version \"{found}\" (expected \"{expected}\")")]
    UnsupportedVersion { expected: String, found: String },
}

// ---------------------------------------------------------------------------
// On-disk envelope (differs from studio_core::Project layout)
// ---------------------------------------------------------------------------

/// The top-level JSON structure as it appears on disk.
///
/// The JSON nests `name` and `seed` inside `"project"`, while the remaining
/// fields (`video`, `assets`, `scenes`, `output`) are siblings. We flatten
/// this into `studio_core::Project` after deserialization.
#[derive(Debug, Deserialize)]
struct VideoDocument {
    schema_version: String,
    project: ProjectMeta,
    video: VideoSettings,
    assets: AssetTable,
    scenes: Vec<Scene>,
    output: OutputSettings,
}

#[derive(Debug, Deserialize)]
struct ProjectMeta {
    name: String,
    #[serde(default = "default_seed")]
    seed: u64,
}

fn default_seed() -> u64 {
    42
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Parse a JSON string into a `Project`.
///
/// # Errors
///
/// Returns [`ParseError::InvalidJson`] if the string is not valid JSON or is
/// missing required fields. Returns [`ParseError::UnsupportedVersion`] if the
/// `schema_version` is not `"1.0"`.
pub fn parse_str(json: &str) -> Result<Project, ParseError> {
    let doc: VideoDocument = serde_json::from_str(json)?;
    check_version(&doc.schema_version)?;
    Ok(into_project(doc))
}

/// Read and parse a JSON video definition from a file path.
///
/// # Errors
///
/// Returns [`ParseError::Io`] if the file cannot be read, plus the same
/// errors as [`parse_str`].
pub fn parse_file(path: &Path) -> Result<Project, ParseError> {
    let contents = std::fs::read_to_string(path)?;
    parse_str(&contents)
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn check_version(found: &str) -> Result<(), ParseError> {
    if found == SCHEMA_VERSION {
        Ok(())
    } else {
        Err(ParseError::UnsupportedVersion {
            expected: SCHEMA_VERSION.to_string(),
            found: found.to_string(),
        })
    }
}

fn into_project(doc: VideoDocument) -> Project {
    Project {
        name: doc.project.name,
        seed: doc.project.seed,
        video: doc.video,
        assets: doc.assets,
        scenes: doc.scenes,
        output: doc.output,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn example_path() -> PathBuf {
        // Resolve relative to the workspace root (Cargo runs tests from the
        // crate directory, but CARGO_MANIFEST_DIR gives us an absolute anchor).
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/hello_cartoon.json")
    }

    #[test]
    fn parse_hello_cartoon_file() {
        let project = parse_file(&example_path()).expect("should parse example file");
        assert_eq!(project.name, "Cute Cartoon Demo");
        assert_eq!(project.seed, 42);
        assert_eq!(project.video.fps, 30);
        assert_eq!(project.video.width, 1920);
        assert_eq!(project.scenes.len(), 1);
        assert_eq!(project.scenes[0].id, "scene_001");
        assert_eq!(project.scenes[0].characters.len(), 1);
        assert_eq!(project.scenes[0].characters[0].actions.len(), 4);
    }

    #[test]
    fn parse_hello_cartoon_str() {
        let json = std::fs::read_to_string(example_path()).unwrap();
        let project = parse_str(&json).expect("should parse example string");
        assert_eq!(project.name, "Cute Cartoon Demo");
    }

    #[test]
    fn error_on_invalid_syntax() {
        let result = parse_str("{ not valid json }");
        let err = result.expect_err("should fail on invalid syntax");
        assert!(
            matches!(err, ParseError::InvalidJson(_)),
            "expected InvalidJson, got: {err}"
        );
    }

    #[test]
    fn error_on_missing_required_field() {
        // Valid JSON but missing `video`, `assets`, `scenes`, `output`.
        let json = r#"{ "schema_version": "1.0", "project": { "name": "X" } }"#;
        let result = parse_str(json);
        let err = result.expect_err("should fail on missing field");
        assert!(
            matches!(err, ParseError::InvalidJson(_)),
            "expected InvalidJson, got: {err}"
        );
        let msg = err.to_string();
        assert!(
            msg.contains("video"),
            "error should mention missing field: {msg}"
        );
    }

    #[test]
    fn error_on_unsupported_version() {
        let json = std::fs::read_to_string(example_path()).unwrap();
        let patched = json.replace("\"1.0\"", "\"99.0\"");
        let result = parse_str(&patched);
        let err = result.expect_err("should fail on bad version");
        assert!(
            matches!(err, ParseError::UnsupportedVersion { .. }),
            "expected UnsupportedVersion, got: {err}"
        );
    }

    #[test]
    fn error_on_file_not_found() {
        let result = parse_file(Path::new("/nonexistent/path.json"));
        let err = result.expect_err("should fail on missing file");
        assert!(matches!(err, ParseError::Io(_)), "expected Io, got: {err}");
    }

    #[test]
    fn schema_version_is_defined() {
        assert_eq!(SCHEMA_VERSION, "1.0");
    }
}
