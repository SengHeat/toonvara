use std::path::{Path, PathBuf};
use std::process;

use clap::{Parser, Subcommand};
use studio_json::ParseError;

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
                report_error(&file, &e);
                process::exit(1);
            }
        },
        Commands::Render { file, out, .. } => {
            let out_display = out
                .as_ref()
                .map_or_else(|| "(from JSON)".to_string(), |p| p.display().to_string());
            eprintln!("toonvara render: {} --out {out_display}", file.display());
            eprintln!("Not yet implemented.");
        }
        Commands::Inspect { path } => {
            eprintln!("toonvara inspect: {}", path.display());
            eprintln!("Not yet implemented.");
        }
        Commands::ListAssets { path } => {
            eprintln!("toonvara list-assets: {}", path.display());
            eprintln!("Not yet implemented.");
        }
    }
}

/// Print a structured, actionable error message to stderr.
fn report_error(file: &Path, err: &ParseError) {
    let path = file.display();
    match err {
        ParseError::Io(e) => {
            eprintln!("error: cannot read {path}");
            eprintln!("  {e}");
        }
        ParseError::InvalidJson(e) => {
            eprintln!("error: invalid JSON in {path}");
            // serde_json errors include line/column and the specific problem
            // (missing field, wrong type, syntax error). Print each piece.
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
