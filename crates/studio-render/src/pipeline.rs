//! End-to-end render pipeline: Blender frame sequence → `FFmpeg` MP4.

use std::path::{Path, PathBuf};

use crate::blender;
use crate::ffmpeg;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors from the render pipeline.
#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    #[error("blender render failed: {0}")]
    Blender(#[from] blender::RenderError),

    #[error("ffmpeg encode failed: {0}")]
    Ffmpeg(#[from] ffmpeg::FfmpegError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("rendered only {found} of {expected} frames")]
    FrameCountMismatch { expected: u32, found: u32 },

    #[error("probe check failed: {detail}")]
    VerifyFailed { detail: String },

    #[error("character GLB not found: {path}")]
    CharacterMissing { path: String },

    #[error("clip validation failed: {0}")]
    ClipValidation(String),

    #[error("audio file not found: {path}")]
    AudioMissing { path: String },
}

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Parameters for the full render-to-MP4 pipeline.
pub struct VideoPipelineParams {
    /// Resolved JSON scenes; when present these take precedence over the single clip.
    pub scenes: Option<Vec<studio_core::Scene>>,
    /// Path to `adapters/blender/render.py`.
    pub blender_script: PathBuf,
    /// Final MP4 output path.
    pub out_path: PathBuf,
    /// Output width in pixels.
    pub width: u32,
    /// Output height in pixels.
    pub height: u32,
    /// Frames per second.
    pub fps: u32,
    /// Total number of frames to render.
    pub frame_count: u32,
    /// Camera position (Y-up).
    pub camera_pos: [f32; 3],
    /// Camera look-at target (Y-up).
    pub camera_target: [f32; 3],
    /// Optional absolute path to a `.glb` character file.
    pub character_glb: Option<PathBuf>,
    /// Character position in Y-up coords.
    pub character_pos: [f32; 3],
    /// Optional animation clip (action) name to play.
    pub animation_clip: Option<String>,
}

/// Successful pipeline result.
#[derive(Debug)]
pub struct VideoPipelineResult {
    /// Path to the final MP4.
    pub output_path: PathBuf,
    /// Size of the MP4 in bytes.
    pub output_size: u64,
    /// Probed metadata from the output.
    pub probe: ffmpeg::ProbeInfo,
    /// Blender stdout + stderr combined.
    pub blender_log: String,
    /// `FFmpeg` stderr (its main log channel).
    pub ffmpeg_log: String,
}

// ---------------------------------------------------------------------------
// Pipeline
// ---------------------------------------------------------------------------

/// Render a frame sequence with Blender, encode to MP4 with `FFmpeg`, verify,
/// and clean up temp frames.
///
/// # Errors
///
/// Returns [`PipelineError`] at any stage if Blender/`FFmpeg` fails, the
/// expected frame count is wrong, or the output doesn't match expectations.
pub fn render_video(params: &VideoPipelineParams) -> Result<VideoPipelineResult, PipelineError> {
    // 1. Create temp dir for frames.
    let temp_dir = std::env::temp_dir().join(format!("toonvara_render_{}", std::process::id()));
    std::fs::create_dir_all(&temp_dir)?;

    let result = render_video_inner(params, &temp_dir);

    // Always attempt cleanup of the temp directory.
    let _ = std::fs::remove_dir_all(&temp_dir);

    result
}

#[allow(clippy::too_many_lines)]
fn render_video_inner(
    params: &VideoPipelineParams,
    temp_dir: &Path,
) -> Result<VideoPipelineResult, PipelineError> {
    let frame_prefix = temp_dir.join("frame_");

    // 2. Detect Blender.
    let blender_info = blender::detect().map_err(blender::RenderError::from)?;

    // 3. Verify adapter script.
    if !params.blender_script.is_file() {
        return Err(blender::RenderError::ScriptNotFound {
            path: params.blender_script.display().to_string(),
        }
        .into());
    }

    // 4. Validate character GLB if provided.
    if let Some(glb) = &params.character_glb {
        if !glb.is_file() {
            return Err(PipelineError::CharacterMissing {
                path: glb.display().to_string(),
            });
        }
    }

    // 5. Validate scheduled clips against GLB contents.
    if let Some(scenes) = &params.scenes {
        let inspect_script = params.blender_script.with_file_name("inspect_glb.py");
        // Collect unique GLB paths for characters that have actions.
        let mut seen = std::collections::HashSet::new();
        let glb_paths: Vec<String> = scenes
            .iter()
            .flat_map(|s| &s.characters)
            .filter(|c| !c.actions.is_empty())
            .filter(|c| seen.insert(c.asset.clone()))
            .map(|c| c.asset.clone())
            .collect();

        if !glb_paths.is_empty() {
            let mut available = std::collections::HashMap::new();
            for glb_path in &glb_paths {
                let info = blender::inspect_glb(&inspect_script, Path::new(glb_path))?;
                let clip_names: Vec<String> = info.actions.iter().map(|a| a.name.clone()).collect();
                eprintln!(
                    "asset {}: {} clip(s): {:?}",
                    glb_path,
                    clip_names.len(),
                    clip_names
                );
                available.insert(glb_path.clone(), clip_names);
            }
            crate::schedule::validate_clips(scenes, &available)
                .map_err(PipelineError::ClipValidation)?;

            // Log the resolved schedule.
            for scene in scenes {
                for character in &scene.characters {
                    for action in &character.actions {
                        let global_start = scene.start_frame + action.start_frame;
                        let global_end = global_start + action.duration_frames;
                        eprintln!(
                            "schedule: {} | {:>10} | frames {:>4}..{:<4} | scene {}",
                            character.id, action.clip, global_start, global_end, scene.id
                        );
                    }
                }
            }
        }
    }

    let mut cmd = render_command(params, &blender_info.path, temp_dir, &frame_prefix)?;

    let blender_output = cmd
        .output()
        .map_err(|e| blender::RenderError::from(blender::BlenderError::from(e)))?;

    let blender_stdout = String::from_utf8_lossy(&blender_output.stdout).into_owned();
    let blender_stderr = String::from_utf8_lossy(&blender_output.stderr).into_owned();
    let blender_log = format!("{blender_stdout}\n{blender_stderr}");

    if !blender_output.status.success() {
        return Err(blender::RenderError::ProcessFailed {
            status: blender_output
                .status
                .code()
                .map_or_else(|| "signal".to_string(), |c| c.to_string()),
            stdout: blender_stdout,
            stderr: blender_stderr,
        }
        .into());
    }

    // 6. Verify all expected frames exist.
    let found = count_frames(temp_dir, "frame_");
    if found < params.frame_count {
        return Err(PipelineError::FrameCountMismatch {
            expected: params.frame_count,
            found,
        });
    }

    // 7. Collect audio tracks from all scenes.
    let audio_tracks = collect_audio_tracks(params);
    let has_audio = !audio_tracks.is_empty();

    // 8. Encode video with FFmpeg.
    let input_pattern = format!("{}%04d.png", frame_prefix.display());
    let video_only_path = if has_audio {
        temp_dir.join("video_only.mp4")
    } else {
        params.out_path.clone()
    };
    let encode_result = ffmpeg::encode(&ffmpeg::EncodeParams {
        input_pattern,
        start_number: 1,
        out_path: video_only_path.clone(),
        fps: params.fps,
    })?;
    let mut ffmpeg_log = encode_result.stderr;

    // 9. Mix all audio tracks and mux into the final MP4.
    if has_audio {
        for track in &audio_tracks {
            if !track.path.is_file() {
                return Err(PipelineError::AudioMissing {
                    path: track.path.display().to_string(),
                });
            }
        }
        for track in &audio_tracks {
            eprintln!(
                "audio: {} (delay={:.2}s, vol={:.2})",
                track.path.display(),
                track.delay_secs,
                track.volume,
            );
        }
        let mix_result = ffmpeg::mix_audio(&ffmpeg::MixAudioParams {
            video_path: video_only_path,
            tracks: audio_tracks,
            out_path: params.out_path.clone(),
        })?;
        // mix_audio returns the filter graph in stdout for logging.
        eprintln!("audio filter_complex: {}", mix_result.stdout);
        ffmpeg_log.push('\n');
        ffmpeg_log.push_str(&mix_result.stderr);
    }

    // 10. Probe the output.
    let probe = ffmpeg::probe(&params.out_path)?;

    // 11. Basic sanity checks.
    if probe.width != params.width || probe.height != params.height {
        return Err(PipelineError::VerifyFailed {
            detail: format!(
                "resolution mismatch: expected {}x{}, got {}x{}",
                params.width, params.height, probe.width, probe.height
            ),
        });
    }

    let expected_duration = f64::from(params.frame_count) / f64::from(params.fps);
    if (probe.duration_secs - expected_duration).abs() > 0.5 {
        return Err(PipelineError::VerifyFailed {
            detail: format!(
                "duration mismatch: expected ~{expected_duration:.2}s, got {:.2}s",
                probe.duration_secs
            ),
        });
    }

    if has_audio && !probe.has_audio {
        return Err(PipelineError::VerifyFailed {
            detail: "audio was mixed but output has no audio stream".to_string(),
        });
    }

    let final_size =
        std::fs::metadata(&params.out_path).map_or(encode_result.output_size, |m| m.len());

    Ok(VideoPipelineResult {
        output_path: params.out_path.clone(),
        output_size: final_size,
        probe,
        blender_log,
        ffmpeg_log,
    })
}

/// Collect all audio clips from resolved scenes as `FFmpeg` `AudioTrack`s.
///
/// Each clip's `start_frame` is converted to a delay in seconds, offset by
/// its scene's global `start_frame`.
fn collect_audio_tracks(params: &VideoPipelineParams) -> Vec<ffmpeg::AudioTrack> {
    let Some(scenes) = &params.scenes else {
        return Vec::new();
    };
    scenes
        .iter()
        .flat_map(|s| {
            s.audio.iter().map(move |clip| {
                #[allow(clippy::cast_precision_loss)]
                let delay_secs = (s.start_frame + clip.start_frame) as f64 / f64::from(params.fps);
                ffmpeg::AudioTrack {
                    path: PathBuf::from(&clip.path),
                    delay_secs,
                    volume: clip.volume,
                }
            })
        })
        .collect()
}

fn render_command(
    params: &VideoPipelineParams,
    executable: &Path,
    temp_dir: &Path,
    frame_prefix: &Path,
) -> Result<std::process::Command, PipelineError> {
    let mut cmd = std::process::Command::new(executable);
    cmd.arg("--background")
        .args(["--python-exit-code", "1"])
        .arg("--python")
        .arg(&params.blender_script)
        .env("TOONVARA_OUT_PATH", frame_prefix.to_string_lossy().as_ref())
        .env("TOONVARA_RES_X", params.width.to_string())
        .env("TOONVARA_RES_Y", params.height.to_string())
        .env("TOONVARA_FRAME_START", "1")
        .env("TOONVARA_FRAME_END", params.frame_count.to_string())
        .env("TOONVARA_FPS", params.fps.to_string())
        .env("TOONVARA_CAM_POS_X", params.camera_pos[0].to_string())
        .env("TOONVARA_CAM_POS_Y", params.camera_pos[1].to_string())
        .env("TOONVARA_CAM_POS_Z", params.camera_pos[2].to_string())
        .env("TOONVARA_CAM_TGT_X", params.camera_target[0].to_string())
        .env("TOONVARA_CAM_TGT_Y", params.camera_target[1].to_string())
        .env("TOONVARA_CAM_TGT_Z", params.camera_target[2].to_string())
        .env("DISPLAY", "");

    cmd.env(
        "TOONVARA_CLIP",
        params.animation_clip.as_deref().unwrap_or(""),
    );
    cmd.env_remove("TOONVARA_CHARACTER_GLB");

    if let Some(glb) = &params.character_glb {
        cmd.env("TOONVARA_CHARACTER_GLB", glb.to_string_lossy().as_ref())
            .env("TOONVARA_CHAR_POS_X", params.character_pos[0].to_string())
            .env("TOONVARA_CHAR_POS_Y", params.character_pos[1].to_string())
            .env("TOONVARA_CHAR_POS_Z", params.character_pos[2].to_string());
    }

    cmd.env_remove("TOONVARA_SCHEDULE_PATH");
    if let Some(scenes) = &params.scenes {
        let manifest = temp_dir.join("schedule.json");
        let json = serde_json::to_vec(scenes).map_err(std::io::Error::other)?;
        std::fs::write(&manifest, &json)?;
        // Verify the manifest was written and contains the expected asset paths.
        let written = std::fs::read(&manifest)?;
        assert_eq!(json, written, "schedule.json mismatch after write");
        eprintln!(
            "schedule manifest: {} ({} bytes)",
            manifest.display(),
            json.len()
        );
        cmd.env("TOONVARA_SCHEDULE_PATH", &manifest);
    }
    Ok(cmd)
}

/// Count files matching `{prefix}NNNN.png` in a directory.
fn count_frames(dir: &Path, prefix: &str) -> u32 {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    let count = entries
        .filter_map(Result::ok)
        .filter(|e| {
            let name = e.file_name();
            let s = name.to_string_lossy();
            s.starts_with(prefix) && s.ends_with(".png")
        })
        .count();
    u32::try_from(count).unwrap_or(u32::MAX)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_frames_empty_dir() {
        let dir = std::env::temp_dir().join("toonvara_count_test_empty");
        let _ = std::fs::create_dir_all(&dir);
        assert_eq!(count_frames(&dir, "frame_"), 0);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn count_frames_nonexistent() {
        assert_eq!(count_frames(Path::new("/no/such/dir"), "frame_"), 0);
    }

    /// Full pipeline integration test.  Run with:
    /// `cargo test -p studio-render -- --ignored pipeline_smoke`
    #[test]
    #[ignore = "requires Blender + FFmpeg installed"]
    fn pipeline_smoke() {
        let script =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../adapters/blender/render.py");
        let out_dir = std::env::temp_dir().join("toonvara_pipeline_test");
        let _ = std::fs::create_dir_all(&out_dir);
        let out_mp4 = out_dir.join("test.mp4");
        let _ = std::fs::remove_file(&out_mp4);

        let params = VideoPipelineParams {
            scenes: None,
            blender_script: script,
            out_path: out_mp4.clone(),
            width: 320,
            height: 240,
            fps: 30,
            frame_count: 30,
            camera_pos: [0.0, 2.0, 6.0],
            camera_target: [0.0, 1.0, 0.0],
            character_glb: None,
            character_pos: [0.0, 0.0, 0.0],
            animation_clip: None,
        };

        let result = render_video(&params).expect("pipeline should succeed");
        assert!(result.output_size > 0, "MP4 must be non-empty");
        assert_eq!(result.probe.width, 320);
        assert_eq!(result.probe.height, 240);
        assert!(
            (result.probe.fps - 30.0).abs() < 1.0,
            "fps should be ~30, got {}",
            result.probe.fps
        );
        assert!(
            result.probe.duration_secs > 0.5,
            "duration should be ~1s, got {}",
            result.probe.duration_secs
        );

        // Clean up.
        let _ = std::fs::remove_file(&out_mp4);
        let _ = std::fs::remove_dir_all(&out_dir);
    }
}
