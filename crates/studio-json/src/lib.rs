//! JSON parsing and validation for Toonvara video definitions.
//!
//! This crate owns the on-disk JSON format. It deserializes the JSON envelope
//! (which has `schema_version` + `project` wrapper) into `studio_core` model
//! types and produces actionable error diagnostics.
//!
//! Semantic cross-field validation (asset resolution, unique IDs, timing
//! bounds) will be added in a later phase.

pub use studio_core::SCHEMA_VERSION;

mod parse;

pub use parse::{parse_file, parse_str, ParseError};
