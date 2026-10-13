use std::path::{Path, PathBuf};
use std::process;
use std::time::Instant;

use clap::{Parser, Subcommand};
use studio_json::ParseError;
use studio_render::pipeline::{PipelineError, VideoPipelineParams};

/// Toonvara — cartoon video generator (CLI)
#[derive(Parser)]
#[command(name = "toonvara", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new Toonvara project
    New {
        /// Project name
        name: String,
        /// Directory to create the project in (defaults to ./<name>)
        #[arg(short, long)]
        dir: Option<PathBuf>,
    },

    /// Validate a JSON video definition
    Validate {
        /// Path to the JSON file
        file: PathBuf,
    },

    /// Render a JSON video definition to an output video
    Render {
        /// Path to the JSON file
        file: PathBuf,
        /// Output file path (overrides the path in JSON)
        #[arg(short, long)]
        out: Option<PathBuf>,
        /// Override output width
        #[arg(long)]
        width: Option<u32>,
        /// Override output height
        #[arg(long)]
        height: Option<u32>,
        /// Override frames per second
        #[arg(long)]
        fps: Option<u32>,
    },

    /// Inspect a project or JSON file and print its metadata
    Inspect {
        /// Path to a JSON file or project directory
        path: PathBuf,
    },

    /// List assets referenced by a project or JSON file
    ListAssets {
        /// Path to a JSON file or project directory
        path: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::New { name, dir } => {
            let target = dir.unwrap_or_else(|| PathBuf::from(&name));
            eprintln!(
                "toonvara new: would create project '{name}' in {}",
                target.display()
            );
            eprintln!("Not yet implemented.");
        }
        Commands::Validate { file } => match studio_json::parse_file(&file) {
            Ok(project) => {
                let chars: usize = project.scenes.iter().map(|s| s.characters.len()).sum();
                println!("valid: {}", file.display());
                println!("  project : {}", project.name);
                println!(
                    "  video   : {}x{} @ {} fps, {}s",
                    project.video.width,
                    project.video.height,
                    project.video.fps,
                    project.video.duration_seconds,
                );
                println!("  scenes  : {}", project.scenes.len());
                println!("  chars   : {chars}");
                println!(
                    "  output  : {} -> {}",
                    project.output.format, project.output.path
                );
            }
            Err(e) => {
                report_parse_error(&file, &e);
                process::exit(1);
            }
        },
        Commands::Render {
            file,
            out,
            width,
            height,
            fps,
        } => {
            run_render(&file, out.as_deref(), width, height, fps);
        }
        Commands::Inspect { path } => {
            // Project / JSON info (best-effort).
            match studio_json::parse_file(&path) {
                Ok(project) => {
                    println!("project: {}", project.name);
                    println!(
                        "  video : {}x{} @ {} fps, {}s",
                        project.video.width,
                        project.video.height,
                        project.video.fps,
                        project.video.duration_seconds,
                    );
                    println!("  scenes: {}", project.scenes.len());
                    println!(
                        "  output: {} -> {}",
                        project.output.format, project.output.path
                    );
                }
                Err(e) => {
                    println!("project: (cannot load {}: {e})", path.display());
                }
            }
            println!();
            print_tool_status();
        }
        Commands::ListAssets { path } => {
            eprintln!("toonvara list-assets: {}", path.display());
            eprintln!("Not yet implemented.");
        }
    }
}

// ---------------------------------------------------------------------------
// Render command
// ---------------------------------------------------------------------------

fn run_render(
    file: &Path,
    out: Option<&Path>,
    width: Option<u32>,
    height: Option<u32>,
    fps: Option<u32>,
) {
    // 1. Parse + validate JSON.
    eprintln!("parsing {}", file.display());
    let project = match studio_json::parse_file(file) {
        Ok(p) => p,
        Err(e) => {
            report_parse_error(file, &e);
            process::exit(1);
        }
    };

    // 2. Resolve effective settings (CLI flags override JSON).
    let eff_width = width.unwrap_or(project.video.width);
    let eff_height = height.unwrap_or(project.video.height);
    let eff_fps = fps.unwrap_or(project.video.fps);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let frame_count = (project.video.duration_seconds * f64::from(eff_fps)).round() as u32;
    let out_path = out.map_or_else(|| PathBuf::from(&project.output.path), Path::to_path_buf);

    // 3. Extract camera from the first scene (or use defaults).
    let (cam_pos, cam_tgt) = project
        .scenes
        .first()
        .and_then(|s| s.camera.as_ref())
        .map_or(([0.0, 2.0, 6.0], [0.0, 1.0, 0.0]), |c| {
            (
                [c.position.x, c.position.y, c.position.z],
                [c.target.x, c.target.y, c.target.z],
            )
        });

    // 4. Locate the adapter script.
    let script = find_adapter_script();

    // 5. Preflight summary.
    eprintln!("project : {}", project.name);
    eprintln!("video   : {eff_width}x{eff_height} @ {eff_fps} fps, {frame_count} frames");
    eprintln!("output  : {}", out_path.display());
    eprintln!();

    // 6. Preflight: verify external tools before starting.
    if let Err(e) = studio_render::blender::detect() {
        eprintln!("error: blender not available");
        eprintln!("  {e}");
        process::exit(1);
    }
    if let Err(e) = studio_render::ffmpeg::detect() {
        eprintln!("error: ffmpeg not available");
        eprintln!("  {e}");
        process::exit(1);
    }

    eprintln!("rendering {frame_count} frames with blender...");
    let start = Instant::now();

    // 7. Run the pipeline.
    let params = VideoPipelineParams {
        blender_script: script,
        out_path: out_path.clone(),
        width: eff_width,
        height: eff_height,
        fps: eff_fps,
        frame_count,
        camera_pos: cam_pos,
        camera_target: cam_tgt,
    };

    match studio_render::pipeline::render_video(&params) {
        Ok(result) => {
            let elapsed = start.elapsed();
            println!("render complete: {}", result.output_path.display());
            println!(
                "  resolution : {}x{}",
                result.probe.width, result.probe.height
            );
            println!("  fps        : {:.1}", result.probe.fps);
            println!("  duration   : {:.2}s", result.probe.duration_secs);
            #[allow(clippy::cast_precision_loss)]
            let size_kb = result.output_size as f64 / 1024.0;
            println!("  file size  : {size_kb:.1} KB");
            println!("  elapsed    : {:.1}s", elapsed.as_secs_f64());
        }
        Err(e) => {
            // Clean up any partial output so we don't leave a broken file.
            let _ = std::fs::remove_file(&out_path);
            report_render_error(&e);
            process::exit(1);
        }
    }
}

/// Locate `adapters/blender/render.py` relative to the running binary, or
/// fall back to the working directory.
fn find_adapter_script() -> PathBuf {
    // When running via `cargo run`, the binary is in target/debug/ — walk up
    // to the workspace root.  When installed, the script should be bundled
    // alongside the binary (future packaging concern).
    let candidates: Vec<PathBuf> = vec![
        // Running from workspace root (common during development).
        PathBuf::from("adapters/blender/render.py"),
        // Binary lives in target/debug or target/release.
        std::env::current_exe()
            .ok()
            .and_then(|p| {
                p.parent()
                    .map(|d| d.join("../../adapters/blender/render.py"))
            })
            .unwrap_or_default(),
    ];

    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }

    // Last resort: let the pipeline report a clear error.
    candidates.into_iter().next().unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Error reporting
// ---------------------------------------------------------------------------

/// Print a structured, actionable JSON-parse error to stderr.
fn report_parse_error(file: &Path, err: &ParseError) {
    let path = file.display();
    match err {
        ParseError::Io(e) => {
            eprintln!("error: cannot read {path}");
            eprintln!("  {e}");
        }
        ParseError::InvalidJson(e) => {
            eprintln!("error: invalid JSON in {path}");
            eprintln!("  line {}, column {}: {}", e.line(), e.column(), e);
            if e.is_syntax() {
                eprintln!(
                    "  hint: check for trailing commas, unquoted keys, or mismatched brackets"
                );
            }
        }
        ParseError::UnsupportedVersion { expected, found } => {
            eprintln!("error: unsupported schema version in {path}");
            eprintln!("  found \"{found}\", expected \"{expected}\"");
            eprintln!(
                "  hint: this version of toonvara only supports schema_version \"{expected}\""
            );
        }
    }
}

/// Print a structured, actionable render-pipeline error to stderr.
fn report_render_error(err: &PipelineError) {
    match err {
        PipelineError::Blender(inner) => {
            eprintln!("error: blender render failed");
            eprintln!("  {inner}");
        }
        PipelineError::Ffmpeg(inner) => {
            eprintln!("error: ffmpeg encode failed");
            eprintln!("  {inner}");
        }
        PipelineError::Io(e) => {
            eprintln!("error: I/O failure during render");
            eprintln!("  {e}");
        }
        PipelineError::FrameCountMismatch { expected, found } => {
            eprintln!("error: blender produced {found} frames, expected {expected}");
            eprintln!("  hint: check blender log output above for render errors");
        }
        PipelineError::VerifyFailed { detail } => {
            eprintln!("error: output verification failed");
            eprintln!("  {detail}");
        }
    }
}

/// Print detected Blender and `FFmpeg` status to stdout.
fn print_tool_status() {
    println!("external tools:");

    match studio_render::blender::detect() {
        Ok(info) => {
            println!("  blender : {} ({})", info.version, info.path.display());
        }
        Err(e) => {
            println!("  blender : NOT AVAILABLE — {e}");
        }
    }

    match studio_render::ffmpeg::detect() {
        Ok(info) => {
            println!("  ffmpeg  : {} ({})", info.version, info.path.display());
        }
        Err(e) => {
            println!("  ffmpeg  : NOT AVAILABLE — {e}");
        }
    }
}
