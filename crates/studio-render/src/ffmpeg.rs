//! `FFmpeg` executable detection, H.264 encoding, and output probing.

use std::path::{Path, PathBuf};
use std::process::Command;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors from locating or running `FFmpeg` / `ffprobe`.
#[derive(Debug, thiserror::Error)]
pub enum FfmpegError {
    #[error("ffmpeg not found on PATH")]
    NotFound,

    #[error("failed to run ffmpeg: {0}")]
    ExecFailed(#[from] std::io::Error),

    #[error("could not parse ffmpeg version from output: {output:?}")]
    VersionParse { output: String },

    #[error(
        "ffmpeg encode failed (exit {status}):\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}"
    )]
    EncodeFailed {
        status: String,
        stdout: String,
        stderr: String,
    },

    #[error("ffprobe failed or returned unparseable output: {detail}")]
    ProbeFailed { detail: String },

    #[error("audio file not found: {path}")]
    AudioMissing { path: String },
}

// ---------------------------------------------------------------------------
// Public types — detection
// ---------------------------------------------------------------------------

/// Information about a detected `FFmpeg` installation.
#[derive(Debug, Clone)]
pub struct FfmpegInfo {
    /// Absolute path to the ffmpeg executable.
    pub path: PathBuf,
    /// Version string as reported by `ffmpeg -version` (e.g. "7.1").
    pub version: String,
}

// ---------------------------------------------------------------------------
// Public types — encode
// ---------------------------------------------------------------------------

/// Parameters for encoding a PNG image sequence into an MP4.
pub struct EncodeParams {
    /// `printf`-style input pattern, e.g. `/tmp/frames/frame_%04d.png`.
    pub input_pattern: String,
    /// First frame number in the sequence.
    pub start_number: u32,
    /// Output MP4 path.
    pub out_path: PathBuf,
    /// Frames per second.
    pub fps: u32,
}

/// Result of a successful `FFmpeg` encode.
#[derive(Debug)]
pub struct EncodeResult {
    pub output_path: PathBuf,
    pub output_size: u64,
    pub stdout: String,
    pub stderr: String,
}

// ---------------------------------------------------------------------------
// Public types — probe
// ---------------------------------------------------------------------------

/// Video metadata extracted by `ffprobe`.
#[derive(Debug, Clone)]
pub struct ProbeInfo {
    pub width: u32,
    pub height: u32,
    /// Duration in seconds.
    pub duration_secs: f64,
    /// Frame rate as a rational string (e.g. "30/1").
    pub r_frame_rate: String,
    /// Frame rate as a float (e.g. 30.0).
    pub fps: f64,
    /// Whether the file contains at least one audio stream.
    pub has_audio: bool,
}

// ---------------------------------------------------------------------------
// Public types — audio mux
// ---------------------------------------------------------------------------

/// Parameters for muxing an audio track into a video-only MP4.
pub struct MuxAudioParams {
    /// Path to the video-only MP4.
    pub video_path: PathBuf,
    /// Path to the audio file (.wav or .mp3).
    pub audio_path: PathBuf,
    /// Output MP4 path (with both streams).
    pub out_path: PathBuf,
    /// Delay before audio starts, in seconds.
    pub delay_secs: f64,
    /// Volume multiplier (0.0–1.0).
    pub volume: f64,
}

// ---------------------------------------------------------------------------
// Detection
// ---------------------------------------------------------------------------

/// Locate the `FFmpeg` executable and report its version.
///
/// # Errors
///
/// Returns [`FfmpegError`] if `FFmpeg` cannot be found, cannot be executed,
/// or its version output cannot be parsed.
pub fn detect() -> Result<FfmpegInfo, FfmpegError> {
    let path = find_executable("ffmpeg")?;
    let raw_output = run_version(&path)?;
    let version = parse_version(&raw_output)?;
    Ok(FfmpegInfo { path, version })
}

// ---------------------------------------------------------------------------
// Encode
// ---------------------------------------------------------------------------

/// Encode a numbered PNG sequence into an H.264 MP4 via `FFmpeg`.
///
/// All arguments are passed as explicit command-line args to the `ffmpeg`
/// binary — no shell is involved (NFR-005).
///
/// # Errors
///
/// Returns [`FfmpegError`] if `FFmpeg` is not found, exits non-zero, or the
/// output file is missing / empty.
pub fn encode(params: &EncodeParams) -> Result<EncodeResult, FfmpegError> {
    let ffmpeg = find_executable("ffmpeg")?;

    // Ensure output directory exists.
    if let Some(parent) = params.out_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let output = Command::new(&ffmpeg)
        .arg("-y") // overwrite without asking
        .arg("-framerate")
        .arg(params.fps.to_string())
        .arg("-start_number")
        .arg(params.start_number.to_string())
        .arg("-i")
        .arg(&params.input_pattern)
        .arg("-c:v")
        .arg("libx264")
        .arg("-pix_fmt")
        .arg("yuv420p")
        // Constant-rate factor for reasonable quality at small size.
        .arg("-crf")
        .arg("23")
        .arg(params.out_path.as_os_str())
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    if !output.status.success() {
        return Err(FfmpegError::EncodeFailed {
            status: output
                .status
                .code()
                .map_or_else(|| "signal".to_string(), |c| c.to_string()),
            stdout,
            stderr,
        });
    }

    let meta = std::fs::metadata(&params.out_path).map_err(|e| FfmpegError::ProbeFailed {
        detail: format!("output missing after encode: {e}"),
    })?;
    if meta.len() == 0 {
        return Err(FfmpegError::ProbeFailed {
            detail: "output file is empty".to_string(),
        });
    }

    Ok(EncodeResult {
        output_path: params.out_path.clone(),
        output_size: meta.len(),
        stdout,
        stderr,
    })
}

// ---------------------------------------------------------------------------
// Audio mux
// ---------------------------------------------------------------------------

/// Combine a video-only MP4 with a single audio file.
///
/// Applies an optional delay (via the `adelay` filter) and volume adjustment
/// (via the `volume` filter). The video stream is copied without re-encoding.
///
/// # Errors
///
/// Returns [`FfmpegError::AudioMissing`] if the audio file does not exist,
/// or [`FfmpegError::EncodeFailed`] if `FFmpeg` exits non-zero.
pub fn mux_audio(params: &MuxAudioParams) -> Result<EncodeResult, FfmpegError> {
    if !params.audio_path.is_file() {
        return Err(FfmpegError::AudioMissing {
            path: params.audio_path.display().to_string(),
        });
    }

    let ffmpeg = find_executable("ffmpeg")?;

    if let Some(parent) = params.out_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Build the audio filter chain: delay then volume.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let delay_ms = (params.delay_secs * 1000.0).round() as u64;
    let vol = params.volume;
    let filter = if delay_ms > 0 && (vol - 1.0).abs() > f64::EPSILON {
        format!("adelay={delay_ms}|{delay_ms},volume={vol}")
    } else if delay_ms > 0 {
        format!("adelay={delay_ms}|{delay_ms}")
    } else if (vol - 1.0).abs() > f64::EPSILON {
        format!("volume={vol}")
    } else {
        String::new()
    };

    let mut cmd = Command::new(&ffmpeg);
    cmd.arg("-y")
        .arg("-i")
        .arg(params.video_path.as_os_str())
        .arg("-i")
        .arg(params.audio_path.as_os_str())
        .arg("-c:v")
        .arg("copy")
        .arg("-map")
        .arg("0:v:0")
        .arg("-map")
        .arg("1:a:0");

    if filter.is_empty() {
        cmd.arg("-c:a").arg("aac");
    } else {
        cmd.arg("-af").arg(&filter);
    }

    // Ensure the output is no shorter than the video.
    cmd.arg("-shortest");
    cmd.arg(params.out_path.as_os_str());

    let output = cmd.output()?;
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    if !output.status.success() {
        return Err(FfmpegError::EncodeFailed {
            status: output
                .status
                .code()
                .map_or_else(|| "signal".to_string(), |c| c.to_string()),
            stdout,
            stderr,
        });
    }

    let meta = std::fs::metadata(&params.out_path).map_err(|e| FfmpegError::ProbeFailed {
        detail: format!("muxed output missing after encode: {e}"),
    })?;

    Ok(EncodeResult {
        output_path: params.out_path.clone(),
        output_size: meta.len(),
        stdout,
        stderr,
    })
}

// ---------------------------------------------------------------------------
// Probe
// ---------------------------------------------------------------------------

/// Probe a video file with `ffprobe` and return basic metadata.
///
/// # Errors
///
/// Returns [`FfmpegError::ProbeFailed`] if `ffprobe` is not found, exits
/// non-zero, or its JSON output cannot be parsed.
pub fn probe(path: &Path) -> Result<ProbeInfo, FfmpegError> {
    let ffprobe = find_executable("ffprobe")?;

    // Query video stream metadata.
    let output = Command::new(&ffprobe)
        .arg("-v")
        .arg("error")
        .arg("-select_streams")
        .arg("v:0")
        .arg("-show_entries")
        .arg("stream=width,height,r_frame_rate:format=duration")
        .arg("-of")
        .arg("json")
        .arg(path.as_os_str())
        .output()?;

    if !output.status.success() {
        return Err(FfmpegError::ProbeFailed {
            detail: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    let mut info = parse_probe_json(&json_str)?;

    // Check for audio stream presence (separate query to avoid mixing streams).
    let audio_output = Command::new(&ffprobe)
        .arg("-v")
        .arg("error")
        .arg("-select_streams")
        .arg("a:0")
        .arg("-show_entries")
        .arg("stream=codec_type")
        .arg("-of")
        .arg("csv=p=0")
        .arg(path.as_os_str())
        .output()?;

    info.has_audio = audio_output.status.success() && !output_is_empty(&audio_output.stdout);

    Ok(info)
}

fn output_is_empty(bytes: &[u8]) -> bool {
    bytes.iter().all(u8::is_ascii_whitespace)
}

/// Parse the JSON emitted by `ffprobe -of json`.
fn parse_probe_json(json: &str) -> Result<ProbeInfo, FfmpegError> {
    // Minimal manual parsing — avoids adding serde_json to this crate just
    // for probe output.  We look for specific key-value patterns.
    let err = || FfmpegError::ProbeFailed {
        detail: format!(
            "cannot parse ffprobe JSON: {}",
            &json[..json.len().min(300)]
        ),
    };

    let width = extract_json_int(json, "width").ok_or_else(err)?;
    let height = extract_json_int(json, "height").ok_or_else(err)?;
    let r_frame_rate = extract_json_str(json, "r_frame_rate").ok_or_else(err)?;
    let duration_str = extract_json_str(json, "duration").ok_or_else(err)?;
    let duration_secs: f64 = duration_str.parse().map_err(|_| err())?;

    let fps = parse_rational_fps(&r_frame_rate).unwrap_or(0.0);

    Ok(ProbeInfo {
        width,
        height,
        duration_secs,
        r_frame_rate,
        fps,
        has_audio: false, // Caller upgrades via the audio-stream query.
    })
}

/// Extract `"key": 123` from a JSON string (integer value).
fn extract_json_int(json: &str, key: &str) -> Option<u32> {
    let needle = format!("\"{key}\"");
    let pos = json.find(&needle)?;
    let after_key = &json[pos + needle.len()..];
    // Skip optional whitespace and colon.
    let after_colon = after_key.trim_start().strip_prefix(':')?;
    let num_str: String = after_colon
        .trim_start()
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    num_str.parse().ok()
}

/// Extract `"key": "value"` from a JSON string.
fn extract_json_str(json: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let pos = json.find(&needle)?;
    let after_key = &json[pos + needle.len()..];
    let after_colon = after_key.trim_start().strip_prefix(':')?;
    let trimmed = after_colon.trim_start();
    if !trimmed.starts_with('"') {
        return None;
    }
    let inner = &trimmed[1..];
    let end = inner.find('"')?;
    Some(inner[..end].to_string())
}

/// Parse "30/1" → 30.0, "30000/1001" → 29.97, etc.
fn parse_rational_fps(s: &str) -> Option<f64> {
    let (num, den) = s.split_once('/')?;
    let n: f64 = num.trim().parse().ok()?;
    let d: f64 = den.trim().parse().ok()?;
    if d == 0.0 {
        return None;
    }
    Some(n / d)
}

// ---------------------------------------------------------------------------
// Internal helpers — detection
// ---------------------------------------------------------------------------

fn find_executable(name: &str) -> Result<PathBuf, FfmpegError> {
    if let Ok(output) = Command::new("which").arg(name).output() {
        if output.status.success() {
            let s = String::from_utf8_lossy(&output.stdout);
            let trimmed = s.trim();
            if !trimmed.is_empty() {
                return Ok(PathBuf::from(trimmed));
            }
        }
    }
    Err(FfmpegError::NotFound)
}

fn run_version(exe: &Path) -> Result<String, FfmpegError> {
    let output = Command::new(exe).arg("-version").output()?;
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Parse a version string from `ffmpeg -version` output.
///
/// Typical first line: `ffmpeg version 7.1 Copyright ...`
/// or: `ffmpeg version N-12345-gabcdef ...`
fn parse_version(raw: &str) -> Result<String, FfmpegError> {
    for line in raw.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("ffmpeg version") {
            let rest = rest.trim();
            let ver: String = rest.chars().take_while(|c| !c.is_whitespace()).collect();
            if !ver.is_empty() {
                return Ok(ver);
            }
        }
    }
    Err(FfmpegError::VersionParse {
        output: raw.chars().take(200).collect(),
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_typical_version() {
        let input = "ffmpeg version 7.1 Copyright (c) 2000-2024\n  built with ...\n";
        let ver = parse_version(input).unwrap();
        assert_eq!(ver, "7.1");
    }

    #[test]
    fn parse_nightly_version() {
        let input = "ffmpeg version N-12345-gabcdef Copyright (c) ...\n";
        let ver = parse_version(input).unwrap();
        assert_eq!(ver, "N-12345-gabcdef");
    }

    #[test]
    fn parse_version_with_patch() {
        let input = "ffmpeg version 6.1.1 Copyright (c) ...\n";
        let ver = parse_version(input).unwrap();
        assert_eq!(ver, "6.1.1");
    }

    #[test]
    fn parse_version_fails_on_garbage() {
        let result = parse_version("not ffmpeg output");
        assert!(matches!(result, Err(FfmpegError::VersionParse { .. })));
    }

    #[test]
    fn parse_rational_fps_works() {
        assert!((parse_rational_fps("30/1").unwrap() - 30.0).abs() < 0.001);
        assert!((parse_rational_fps("30000/1001").unwrap() - 29.97).abs() < 0.01);
        assert!(parse_rational_fps("0/0").is_none());
        assert!(parse_rational_fps("garbage").is_none());
    }

    #[test]
    fn extract_json_int_works() {
        let json = r#"{"width": 320, "height": 240}"#;
        assert_eq!(extract_json_int(json, "width"), Some(320));
        assert_eq!(extract_json_int(json, "height"), Some(240));
        assert_eq!(extract_json_int(json, "missing"), None);
    }

    #[test]
    fn extract_json_str_works() {
        let json = r#"{"duration": "1.033333", "r_frame_rate": "30/1"}"#;
        assert_eq!(
            extract_json_str(json, "duration"),
            Some("1.033333".to_string())
        );
        assert_eq!(
            extract_json_str(json, "r_frame_rate"),
            Some("30/1".to_string())
        );
    }

    #[test]
    fn parse_probe_json_works() {
        let json = r#"{
            "streams": [{"width": 320, "height": 240, "r_frame_rate": "30/1"}],
            "format": {"duration": "1.000000"}
        }"#;
        let info = parse_probe_json(json).unwrap();
        assert_eq!(info.width, 320);
        assert_eq!(info.height, 240);
        assert!((info.duration_secs - 1.0).abs() < 0.01);
        assert!((info.fps - 30.0).abs() < 0.01);
        assert!(!info.has_audio); // parse_probe_json defaults to false
    }
}
