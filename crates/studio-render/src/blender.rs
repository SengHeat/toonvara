//! Blender executable detection, version checking, and headless rendering.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Minimum supported Blender major.minor version.
pub const MIN_VERSION: (u32, u32) = (3, 0);

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Errors from locating or interrogating Blender.
#[derive(Debug, thiserror::Error)]
pub enum BlenderError {
    #[error("blender not found (searched PATH and common install locations)")]
    NotFound,

    #[error("failed to run blender: {0}")]
    ExecFailed(#[from] std::io::Error),

    #[error("could not parse blender version from output: {output:?}")]
    VersionParse { output: String },

    #[error(
        "blender {found} is too old (minimum supported: {}.{})",
        MIN_VERSION.0,
        MIN_VERSION.1
    )]
    TooOld { found: String },
}

/// Errors from a Blender render invocation.
#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("blender detection failed: {0}")]
    Detection(#[from] BlenderError),

    #[error("adapter script not found: {path}")]
    ScriptNotFound { path: String },

    #[error("blender process failed (exit {status}):\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}")]
    ProcessFailed {
        status: String,
        stdout: String,
        stderr: String,
    },

    #[error("output file missing or empty after render: {path}")]
    OutputMissing { path: String },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

// ---------------------------------------------------------------------------
// Public types — detection
// ---------------------------------------------------------------------------

/// Information about a detected Blender installation.
#[derive(Debug, Clone)]
pub struct BlenderInfo {
    /// Absolute path to the blender executable.
    pub path: PathBuf,
    /// Full version string as reported by `blender --version` (e.g. "4.2.0").
    pub version: String,
    /// Parsed (major, minor, patch).
    pub version_tuple: (u32, u32, u32),
}

// ---------------------------------------------------------------------------
// Public types — render request / result
// ---------------------------------------------------------------------------

/// Parameters for a single-frame render.
pub struct RenderParams {
    /// Path to `adapters/blender/render.py`.
    pub script_path: PathBuf,
    /// Absolute path for the output PNG.
    pub out_path: PathBuf,
    /// Output width in pixels.
    pub width: u32,
    /// Output height in pixels.
    pub height: u32,
    /// Camera position (Y-up, right-handed).
    pub camera_pos: [f32; 3],
    /// Camera look-at target (Y-up, right-handed).
    pub camera_target: [f32; 3],
}

/// Successful render result.
#[derive(Debug)]
pub struct RenderResult {
    /// Path to the rendered PNG.
    pub output_path: PathBuf,
    /// Size of the output file in bytes.
    pub output_size: u64,
    /// Combined stdout from Blender.
    pub stdout: String,
    /// Combined stderr from Blender.
    pub stderr: String,
}

// ---------------------------------------------------------------------------
// Detection (unchanged)
// ---------------------------------------------------------------------------

/// Locate the Blender executable and verify its version.
///
/// Search order:
/// 1. `$BLENDER_PATH` environment variable (if set)
/// 2. `blender` on `$PATH`
/// 3. Common macOS application bundle path
///
/// # Errors
///
/// Returns [`BlenderError`] if Blender cannot be found, cannot be executed,
/// its version output cannot be parsed, or the version is below the minimum.
pub fn detect() -> Result<BlenderInfo, BlenderError> {
    let path = find_executable()?;
    let raw_output = run_version(&path)?;
    let (version, tuple) = parse_version(&raw_output)?;
    check_minimum(tuple, &version)?;
    Ok(BlenderInfo {
        path,
        version,
        version_tuple: tuple,
    })
}

// ---------------------------------------------------------------------------
// Single-frame render
// ---------------------------------------------------------------------------

/// Render a single frame via Blender in headless mode.
///
/// Launches `blender --background --python <script>` with all scene
/// parameters passed through environment variables (no shell interpolation,
/// per NFR-005).
///
/// # Errors
///
/// Returns [`RenderError`] if Blender cannot be found, the adapter script is
/// missing, the process exits non-zero, or the output file is missing/empty.
pub fn render_single_frame(params: &RenderParams) -> Result<RenderResult, RenderError> {
    // 1. Detect Blender.
    let info = detect()?;

    // 2. Verify the adapter script exists.
    if !params.script_path.is_file() {
        return Err(RenderError::ScriptNotFound {
            path: params.script_path.display().to_string(),
        });
    }

    // 3. Ensure output parent directory exists.
    if let Some(parent) = params.out_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // 4. Build the command — only fixed args, data via env vars.
    let output = Command::new(&info.path)
        .arg("--background")
        .arg("--python")
        .arg(&params.script_path)
        // Scene parameters via environment — safe, no injection possible.
        .env("TOONVARA_OUT_PATH", params.out_path.to_string_lossy().as_ref())
        .env("TOONVARA_RES_X", params.width.to_string())
        .env("TOONVARA_RES_Y", params.height.to_string())
        .env("TOONVARA_CAM_POS_X", params.camera_pos[0].to_string())
        .env("TOONVARA_CAM_POS_Y", params.camera_pos[1].to_string())
        .env("TOONVARA_CAM_POS_Z", params.camera_pos[2].to_string())
        .env("TOONVARA_CAM_TGT_X", params.camera_target[0].to_string())
        .env("TOONVARA_CAM_TGT_Y", params.camera_target[1].to_string())
        .env("TOONVARA_CAM_TGT_Z", params.camera_target[2].to_string())
        // Suppress Blender GUI on Linux.
        .env("DISPLAY", "")
        .output()
        .map_err(BlenderError::from)?;

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // 5. Check process exit status.
    if !output.status.success() {
        return Err(RenderError::ProcessFailed {
            status: output
                .status
                .code()
                .map_or_else(|| "signal".to_string(), |c| c.to_string()),
            stdout,
            stderr,
        });
    }

    // 6. Verify output file exists and is non-empty.
    let meta = std::fs::metadata(&params.out_path).map_err(|_| RenderError::OutputMissing {
        path: params.out_path.display().to_string(),
    })?;
    if meta.len() == 0 {
        return Err(RenderError::OutputMissing {
            path: params.out_path.display().to_string(),
        });
    }

    Ok(RenderResult {
        output_path: params.out_path.clone(),
        output_size: meta.len(),
        stdout,
        stderr,
    })
}

// ---------------------------------------------------------------------------
// Internal helpers — detection
// ---------------------------------------------------------------------------

/// Well-known macOS paths to check when `blender` is not on PATH.
const MACOS_CANDIDATES: &[&str] = &[
    "/Applications/Blender.app/Contents/MacOS/Blender",
    "/Applications/Blender.app/Contents/MacOS/blender",
];

fn find_executable() -> Result<PathBuf, BlenderError> {
    // 1. Explicit env var.
    if let Ok(p) = std::env::var("BLENDER_PATH") {
        let path = PathBuf::from(&p);
        if path.is_file() {
            return Ok(path);
        }
    }

    // 2. On PATH — try running it.
    if let Ok(output) = Command::new("which").arg("blender").output() {
        if output.status.success() {
            let s = String::from_utf8_lossy(&output.stdout);
            let trimmed = s.trim();
            if !trimmed.is_empty() {
                return Ok(PathBuf::from(trimmed));
            }
        }
    }

    // 3. Common install locations (macOS).
    for candidate in MACOS_CANDIDATES {
        let p = Path::new(candidate);
        if p.is_file() {
            return Ok(p.to_path_buf());
        }
    }

    Err(BlenderError::NotFound)
}

fn run_version(exe: &Path) -> Result<String, BlenderError> {
    let output = Command::new(exe)
        .arg("--version")
        .env("DISPLAY", "") // avoid GUI pop-up on Linux
        .output()?;
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Parse a version string out of `blender --version` output.
///
/// Typical first line: `Blender 4.2.0`
fn parse_version(raw: &str) -> Result<(String, (u32, u32, u32)), BlenderError> {
    for line in raw.lines() {
        let line = line.trim();
        // Look for "Blender X.Y.Z" or "Blender X.Y"
        if let Some(rest) = line.strip_prefix("Blender") {
            let rest = rest.trim();
            // Take only the version part (stop at space/parenthesis)
            let ver_str: String = rest
                .chars()
                .take_while(|c| *c == '.' || c.is_ascii_digit())
                .collect();
            let parts: Vec<&str> = ver_str.split('.').collect();
            if parts.len() >= 2 {
                let major = parts[0].parse::<u32>();
                let minor = parts[1].parse::<u32>();
                let patch = parts
                    .get(2)
                    .and_then(|p| p.parse::<u32>().ok())
                    .unwrap_or(0);
                if let (Ok(maj), Ok(min)) = (major, minor) {
                    return Ok((ver_str, (maj, min, patch)));
                }
            }
        }
    }
    Err(BlenderError::VersionParse {
        output: raw.chars().take(200).collect(),
    })
}

fn check_minimum(tuple: (u32, u32, u32), found_str: &str) -> Result<(), BlenderError> {
    if (tuple.0, tuple.1) >= MIN_VERSION {
        Ok(())
    } else {
        Err(BlenderError::TooOld {
            found: found_str.to_string(),
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_typical_version() {
        let input = "Blender 4.2.0\n  build date: 2024-07-16\n";
        let (ver, tuple) = parse_version(input).unwrap();
        assert_eq!(ver, "4.2.0");
        assert_eq!(tuple, (4, 2, 0));
    }

    #[test]
    fn parse_two_part_version() {
        let input = "Blender 3.6\n";
        let (ver, tuple) = parse_version(input).unwrap();
        assert_eq!(ver, "3.6");
        assert_eq!(tuple, (3, 6, 0));
    }

    #[test]
    fn parse_version_with_extra_text() {
        let input = "Blender 4.0.2 (hash abc123)\n";
        let (ver, tuple) = parse_version(input).unwrap();
        assert_eq!(ver, "4.0.2");
        assert_eq!(tuple, (4, 0, 2));
    }

    #[test]
    fn parse_version_fails_on_garbage() {
        let result = parse_version("no version here");
        assert!(matches!(result, Err(BlenderError::VersionParse { .. })));
    }

    #[test]
    fn check_minimum_passes() {
        assert!(check_minimum((3, 0, 0), "3.0.0").is_ok());
        assert!(check_minimum((4, 2, 1), "4.2.1").is_ok());
    }

    #[test]
    fn check_minimum_fails() {
        let err = check_minimum((2, 93, 0), "2.93.0").unwrap_err();
        assert!(matches!(err, BlenderError::TooOld { .. }));
    }

    #[test]
    fn render_fails_with_missing_script() {
        let params = RenderParams {
            script_path: PathBuf::from("/nonexistent/render.py"),
            out_path: PathBuf::from("/tmp/toonvara_test_out.png"),
            width: 320,
            height: 240,
            camera_pos: [0.0, 2.0, 6.0],
            camera_target: [0.0, 1.0, 0.0],
        };
        let err = render_single_frame(&params);
        // Either ScriptNotFound (if Blender is installed) or Detection
        // (if Blender is absent). Both are correct.
        assert!(err.is_err());
    }

    /// Integration test — requires Blender installed.  Run with:
    /// `cargo test -p studio-render -- --ignored render_single_frame_smoke`
    #[test]
    #[ignore = "requires Blender installed"]
    fn render_single_frame_smoke() {
        let script =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../adapters/blender/render.py");
        let out_dir = std::env::temp_dir().join("toonvara_test");
        let _ = std::fs::create_dir_all(&out_dir);
        let out = out_dir.join("smoke_frame.png");
        let _ = std::fs::remove_file(&out);

        let params = RenderParams {
            script_path: script,
            out_path: out.clone(),
            width: 320,
            height: 240,
            camera_pos: [0.0, 2.0, 6.0],
            camera_target: [0.0, 1.0, 0.0],
        };

        let result = render_single_frame(&params).expect("render should succeed");
        assert!(result.output_size > 0, "PNG must be non-empty");
        assert!(out.exists(), "output file must exist");

        // Clean up.
        let _ = std::fs::remove_file(&out);
        let _ = std::fs::remove_dir(&out_dir);
    }
}
