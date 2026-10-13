//! Resolve character assets and validate the supported base-animation timeline.

use std::collections::HashSet;
use std::path::Path;

use studio_core::{CharacterAction, Project, Scene};

/// Produce a sorted render manifest. Scene starts are global; action starts
/// are relative to their scene. All intervals are half-open.
pub fn resolve(project: &Project, base: &Path, frames: u32) -> Result<Vec<Scene>, String> {
    let mut scenes = project.scenes.clone();
    scenes.sort_by_key(|scene| scene.start_frame);
    let mut previous_end = 0;
    let mut ids = HashSet::new();
    for scene in &mut scenes {
        let end = interval_end(scene.start_frame, scene.duration_frames, u64::from(frames))?;
        if scene.start_frame < previous_end || !ids.insert(scene.id.clone()) {
            return Err(format!(
                "scene {} overlaps another scene or repeats its ID",
                scene.id
            ));
        }
        previous_end = end;
        let mut character_ids = HashSet::new();
        for character in &mut scene.characters {
            if !character_ids.insert(character.id.clone()) {
                return Err(format!(
                    "duplicate character ID {} in {}",
                    character.id, scene.id
                ));
            }
            let asset = project
                .assets
                .characters
                .get(&character.asset)
                .ok_or_else(|| format!("unknown character asset: {}", character.asset))?;
            let path = base
                .join(asset)
                .canonicalize()
                .map_err(|e| format!("character asset {asset}: {e}"))?;
            character.asset = path.to_string_lossy().into_owned();
            character.actions.sort_by_key(|action| action.start_frame);
            validate_actions(&character.actions, scene.duration_frames)
                .map_err(|error| format!("character {}: {error}", character.id))?;
        }
    }
    Ok(scenes)
}

fn validate_actions(actions: &[CharacterAction], scene_duration: u64) -> Result<(), String> {
    let mut previous_end = 0;
    let mut earlier_end = 0;
    for action in actions {
        let end = interval_end(action.start_frame, action.duration_frames, scene_duration)?;
        if !action.speed.is_finite() || action.speed <= 0.0 {
            return Err(format!(
                "clip {}: speed must be finite and greater than zero",
                action.clip
            ));
        }
        if action.blend_in_frames >= action.duration_frames {
            return Err("blend_in_frames must be less than duration_frames".to_owned());
        }
        if action.start_frame < previous_end
            && (previous_end - action.start_frame != action.blend_in_frames
                || action.start_frame < earlier_end)
        {
            return Err(format!(
                "overlapping base clips require an exact two-clip crossfade: {}",
                action.clip
            ));
        }
        if action.clip.is_empty() || action.fallback_clip.as_deref() == Some("") {
            return Err("empty clip name".to_owned());
        }
        earlier_end = previous_end;
        previous_end = end;
    }
    Ok(())
}

fn interval_end(start: u64, duration: u64, limit: u64) -> Result<u64, String> {
    start
        .checked_add(duration)
        .filter(|end| duration > 0 && *end <= limit)
        .ok_or_else(|| {
            format!("invalid interval [{start}, +{duration}): must fit within {limit} frames")
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project() -> Project {
        serde_json::from_value(serde_json::json!({
            "name": "schedule test", "video": {"duration_seconds": 15.0, "fps": 30, "width": 320, "height": 240},
            "assets": {"characters": {"hero": "Cargo.toml"}},
            "output": {"format": "mp4", "path": "test.mp4"},
            "scenes": [{"id": "scene", "start_frame": 0, "duration_frames": 450,
                "characters": [{"id": "hero_01", "asset": "hero", "actions": [
                    {"clip": "wave", "start_frame": 120, "duration_frames": 60, "fallback_clip": "idle"},
                    {"clip": "walk", "start_frame": 0, "duration_frames": 120}
                ]}]}]
        })).unwrap()
    }

    #[test]
    fn resolves_paths_sorts_actions_and_preserves_fallback() {
        let scenes = resolve(&project(), Path::new(env!("CARGO_MANIFEST_DIR")), 450).unwrap();
        let character = &scenes[0].characters[0];
        assert!(Path::new(&character.asset).is_absolute());
        assert_eq!(character.actions[0].clip, "walk");
        assert_eq!(character.actions[1].fallback_clip.as_deref(), Some("idle"));
    }

    #[test]
    fn rejects_overlaps_and_actions_outside_scene() {
        for start in [119, 449] {
            let mut input = project();
            input.scenes[0].characters[0].actions[0].start_frame = start;
            assert!(resolve(&input, Path::new(env!("CARGO_MANIFEST_DIR")), 450).is_err());
        }
    }

    #[test]
    fn rejects_unknown_assets_duplicate_ids_and_short_video() {
        let base = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut input = project();
        assert!(resolve(&input, base, 449).is_err());
        let duplicate = input.scenes[0].characters[0].clone();
        input.scenes[0].characters.push(duplicate);
        assert!(resolve(&input, base, 450).is_err());
        input.scenes[0].characters.pop();
        input.assets.characters.clear();
        assert!(resolve(&input, base, 450).is_err());
    }

    #[test]
    fn intervals_are_positive_bounded_and_overflow_checked() {
        assert_eq!(interval_end(120, 60, 450), Ok(180));
        assert!(interval_end(0, 0, 450).is_err());
        assert!(interval_end(449, 2, 450).is_err());
        assert!(interval_end(u64::MAX, 1, u64::MAX).is_err());
    }

    #[test]
    fn speed_does_not_change_action_boundaries() {
        for speed in [0.5, 1.0, 1.5, 2.0] {
            let mut input = project();
            input.scenes[0].characters[0].actions[0].speed = speed;
            let scenes = resolve(&input, Path::new(env!("CARGO_MANIFEST_DIR")), 450).unwrap();
            let action = &scenes[0].characters[0].actions[1];
            assert_eq!((action.start_frame, action.duration_frames), (120, 60));
            assert!((action.speed - speed).abs() < f64::EPSILON);
        }
    }

    #[test]
    fn invalid_programmatic_speeds_are_rejected() {
        for speed in [0.0, -0.5, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut input = project();
            input.scenes[0].characters[0].actions[0].speed = speed;
            let error = resolve(&input, Path::new(env!("CARGO_MANIFEST_DIR")), 450).unwrap_err();
            assert!(error.contains("speed"));
        }
    }

    #[test]
    fn explicit_crossfades_preserve_both_intervals() {
        let mut input = project();
        let incoming = &mut input.scenes[0].characters[0].actions[0];
        incoming.start_frame = 100;
        incoming.blend_in_frames = 20;
        incoming.speed = 2.0;
        let scenes = resolve(&input, Path::new(env!("CARGO_MANIFEST_DIR")), 450).unwrap();
        assert_eq!(scenes[0].characters[0].actions[0].duration_frames, 120);
        assert_eq!(scenes[0].characters[0].actions[1].start_frame, 100);
        input.scenes[0].characters[0].actions[0].blend_in_frames = 19;
        assert!(resolve(&input, Path::new(env!("CARGO_MANIFEST_DIR")), 450).is_err());
        input.scenes[0].characters[0].actions[0].blend_in_frames = 60;
        assert!(resolve(&input, Path::new(env!("CARGO_MANIFEST_DIR")), 450).is_err());
    }

    #[test]
    fn three_way_overlaps_are_rejected() {
        let mut actions = project().scenes.remove(0).characters.remove(0).actions;
        actions.sort_by_key(|action| action.start_frame);
        actions[1].start_frame = 100;
        actions[1].blend_in_frames = 20;
        let mut third = actions[1].clone();
        third.start_frame = 110;
        third.blend_in_frames = 50;
        actions.push(third);
        assert!(validate_actions(&actions, 450).is_err());
    }
}
