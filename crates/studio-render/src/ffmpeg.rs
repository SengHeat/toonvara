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

/// A single audio track to be mixed into the video.
pub struct AudioTrack {
    /// Path to the audio file (.wav or .mp3).
    pub path: PathBuf,
    /// Delay before audio starts, in seconds.
    pub delay_secs: f64,
    /// Volume multiplier (0.0–1.0).
    pub volume: f64,
}

/// Parameters for mixing one or more audio tracks into a video-only MP4.
pub struct MixAudioParams {
    /// Path to the video-only MP4.
    pub video_path: PathBuf,
    /// Audio tracks to mix together.
    pub tracks: Vec<AudioTrack>,
    /// Output MP4 path (with both video and mixed audio).
    pub out_path: PathBuf,
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

/// Mix one or more audio tracks into a video-only MP4.
///
/// Each track gets its own per-track `adelay` (delay) and `volume` filter,
/// then all tracks are combined with `amix`.  The video stream is copied
/// without re-encoding.
///
/// The `filter_complex` graph for N tracks looks like:
///
/// ```text
/// [1:a]adelay=0|0,volume=0.6[a0];
/// [2:a]adelay=1000|1000,volume=1[a1];
/// [a0][a1]amix=inputs=2:normalize=0[aout]
/// ```
///
/// `normalize=0` disables `amix`'s default per-input 1/N scaling so that
/// each track's `volume` is the only gain control.  Returns the filter
/// graph string in [`EncodeResult::stdout`] for logging.
///
/// # Errors
///
/// Returns [`FfmpegError::AudioMissing`] if any audio file does not exist,
/// or [`FfmpegError::EncodeFailed`] if `FFmpeg` exits non-zero.
pub fn mix_audio(params: &MixAudioParams) -> Result<EncodeResult, FfmpegError> {
    for track in &params.tracks {
        if !track.path.is_file() {
            return Err(FfmpegError::AudioMissing {
                path: track.path.display().to_string(),
            });
        }
    }

    let ffmpeg = find_executable("ffmpeg")?;

    if let Some(parent) = params.out_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let filter_graph = build_audio_filter_graph(&params.tracks);

    let mut cmd = Command::new(&ffmpeg);
    cmd.arg("-y").arg("-i").arg(params.video_path.as_os_str());

    // Add each audio file as a separate input.
    for track in &params.tracks {
        cmd.arg("-i").arg(track.path.as_os_str());
    }

    cmd.arg("-filter_complex")
        .arg(&filter_graph)
        .arg("-map")
        .arg("0:v:0")
        .arg("-map")
        .arg("[aout]")
        .arg("-c:v")
        .arg("copy")
        .arg("-c:a")
        .arg("aac")
        .arg("-shortest")
        .arg(params.out_path.as_os_str());

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
        // Return the filter graph so the pipeline can log it.
        stdout: filter_graph,
        stderr,
    })
}

/// Build the `filter_complex` string for N audio tracks.
///
/// Each track (input 1..N, since input 0 is the video) gets per-track
/// `adelay` + `volume` filters, then all are combined with `amix`.
///
/// `amix` defaults to `normalize=1` which scales each input by 1/N — we
/// disable this (`normalize=0`) so each track's explicit `volume` value is
/// the sole gain control.
fn build_audio_filter_graph(tracks: &[AudioTrack]) -> String {
    use std::fmt::Write;

    let mut parts = Vec::new();
    let mut labels = Vec::new();

    for (i, track) in tracks.iter().enumerate() {
        let input_idx = i + 1; // input 0 is the video
        let label = format!("a{i}");

        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let delay_ms = (track.delay_secs * 1000.0).round() as u64;
        let vol = track.volume;

        let mut chain = format!("[{input_idx}:a]");
        if delay_ms > 0 {
            let _ = write!(chain, "adelay={delay_ms}|{delay_ms},");
        }
        let _ = write!(chain, "volume={vol}[{label}]");

        parts.push(chain);
        labels.push(format!("[{label}]"));
    }

    // amix combines all labelled streams into [aout].
    let amix = format!(
        "{}amix=inputs={}:normalize=0[aout]",
        labels.join(""),
        tracks.len()
    );
    parts.push(amix);
    parts.join(";")
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
    fn audio_filter_graph_two_tracks() {
        let tracks = vec![
            AudioTrack {
                path: PathBuf::from("bgm.mp3"),
                delay_secs: 0.0,
                volume: 0.6,
            },
            AudioTrack {
                path: PathBuf::from("voice.wav"),
                delay_secs: 1.0,
                volume: 1.0,
            },
        ];
        let graph = build_audio_filter_graph(&tracks);
        assert_eq!(
            graph,
            "[1:a]volume=0.6[a0];[2:a]adelay=1000|1000,volume=1[a1];[a0][a1]amix=inputs=2:normalize=0[aout]"
        );
    }

    #[test]
    fn audio_filter_graph_single_track() {
        let tracks = vec![AudioTrack {
            path: PathBuf::from("sfx.wav"),
            delay_secs: 2.5,
            volume: 0.8,
        }];
        let graph = build_audio_filter_graph(&tracks);
        assert_eq!(
            graph,
            "[1:a]adelay=2500|2500,volume=0.8[a0];[a0]amix=inputs=1:normalize=0[aout]"
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
