use glam::Vec3;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Top-level project
// ---------------------------------------------------------------------------

/// A complete Toonvara project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    #[serde(default = "default_seed")]
    pub seed: u64,
    pub video: VideoSettings,
    pub assets: AssetTable,
    pub scenes: Vec<Scene>,
    pub output: OutputSettings,
}

fn default_seed() -> u64 {
    42
}

// ---------------------------------------------------------------------------
// Video / output settings
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoSettings {
    pub duration_seconds: f64,
    pub fps: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputSettings {
    pub format: OutputFormat,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Mp4,
}

// ---------------------------------------------------------------------------
// Assets
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AssetTable {
    #[serde(default)]
    pub characters: std::collections::HashMap<String, String>,
}

/// Metadata for a registered character asset (not yet used in JSON, but part of
/// the internal model per README section 7).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterAsset {
    pub id: Uuid,
    pub display_name: String,
    pub category: CharacterCategory,
    pub model_path: String,
    pub animation_clips: Vec<String>,
    pub expression_names: Vec<String>,
    #[serde(default)]
    pub asset_version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CharacterCategory {
    Human,
    Animal,
    Fantasy,
}

// ---------------------------------------------------------------------------
// Scene
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    pub id: String,
    pub start_frame: u64,
    pub duration_frames: u64,
    #[serde(default)]
    pub environment: Option<String>,
    #[serde(default)]
    pub characters: Vec<CharacterInstance>,
    #[serde(default)]
    pub camera: Option<Camera>,
}

// ---------------------------------------------------------------------------
// Character instance (scene-specific)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterInstance {
    pub id: String,
    pub asset: String,
    #[serde(default = "default_vec3")]
    pub position: Vec3,
    #[serde(default = "default_vec3")]
    pub rotation: Vec3,
    #[serde(default = "default_scale")]
    pub scale: Vec3,
    #[serde(default = "default_true")]
    pub visible: bool,
    #[serde(default)]
    pub actions: Vec<CharacterAction>,
    #[serde(default)]
    pub current_expression: Option<String>,
}

fn default_vec3() -> Vec3 {
    Vec3::ZERO
}

fn default_scale() -> Vec3 {
    Vec3::ONE
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterAction {
    pub clip: String,
    pub start_frame: u64,
    pub duration_frames: u64,
}

// ---------------------------------------------------------------------------
// Camera
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Camera {
    #[serde(default = "default_projection")]
    pub projection: CameraProjection,
    pub position: Vec3,
    pub target: Vec3,
    #[serde(default = "default_fov")]
    pub fov: f32,
}

fn default_projection() -> CameraProjection {
    CameraProjection::Perspective
}

fn default_fov() -> f32 {
    60.0
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CameraProjection {
    Perspective,
    Orthographic,
}

// ---------------------------------------------------------------------------
// Schema version (used by studio-json, defined here so core owns the constant)
// ---------------------------------------------------------------------------

pub const SCHEMA_VERSION: &str = "1.0";

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_output_format_roundtrips() {
        let json = serde_json::to_string(&OutputFormat::Mp4).unwrap();
        let back: OutputFormat = serde_json::from_str(&json).unwrap();
        assert_eq!(back, OutputFormat::Mp4);
    }

    #[test]
    fn camera_projection_roundtrips() {
        let json = serde_json::to_string(&CameraProjection::Perspective).unwrap();
        assert_eq!(json, "\"perspective\"");
        let back: CameraProjection = serde_json::from_str(&json).unwrap();
        assert_eq!(back, CameraProjection::Perspective);
    }

    #[test]
    fn character_category_roundtrips() {
        for cat in [
            CharacterCategory::Human,
            CharacterCategory::Animal,
            CharacterCategory::Fantasy,
        ] {
            let json = serde_json::to_string(&cat).unwrap();
            let back: CharacterCategory = serde_json::from_str(&json).unwrap();
            assert_eq!(back, cat);
        }
    }

    #[test]
    fn vec3_serialises_as_array() {
        let v = Vec3::new(1.0, 2.0, 3.0);
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(json, "[1.0,2.0,3.0]");
    }
}
