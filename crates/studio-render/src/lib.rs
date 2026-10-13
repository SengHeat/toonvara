//! Rendering orchestration for Toonvara.
//!
//! This crate owns the integration with external tools (Blender, `FFmpeg`) and
//! will eventually drive the full render pipeline. For now it provides
//! detection and version-checking of those tools.

pub mod blender;
pub mod ffmpeg;
pub mod pipeline;
pub mod schedule;
