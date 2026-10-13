//! Core data model types for Toonvara.
//!
//! These types represent the canonical in-memory project model shared by both
//! the CLI renderer and the desktop application. They map closely to the JSON
//! video specification (see `studio-json`), but are the *internal* representation
//! — the JSON crate is responsible for parsing/serialising the on-disk format
//! into these types.

pub mod model;

pub use model::*;
