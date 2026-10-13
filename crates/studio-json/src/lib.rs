//! JSON schema validation and parsing for Toonvara video definitions.
//!
//! # Planned responsibilities
//!
//! - Parse the on-disk JSON video format (see `schemas/video-v1.schema.json`)
//! - Validate against the versioned schema
//! - Resolve asset references
//! - Convert the parsed representation into `studio_core` model types
//! - Produce actionable error diagnostics
//!
//! # Status
//!
//! **Scaffold only.** The JSON schema definition and validation logic will be
//! implemented in Phase 2. This crate currently re-exports the schema version
//! constant from `studio-core` and provides a placeholder validation entry
//! point.

pub use studio_core::SCHEMA_VERSION;

/// Placeholder: validate a JSON string against the Toonvara video schema.
///
/// Returns `Ok(())` today — real validation comes in Phase 2.
///
/// # Errors
///
/// Will return validation errors once the schema is implemented.
pub fn validate_json(_input: &str) -> Result<(), String> {
    // TODO(phase-2): implement schema validation
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_validation_succeeds() {
        assert!(validate_json("{}").is_ok());
    }

    #[test]
    fn schema_version_is_defined() {
        assert_eq!(SCHEMA_VERSION, "1.0");
    }
}
