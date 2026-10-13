use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

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

fn main() -> Result<()> {
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
        Commands::Validate { file } => {
            eprintln!("toonvara validate: {}", file.display());
            let contents = std::fs::read_to_string(&file)?;
            // Phase 0: just parse to confirm valid JSON, real validation in Phase 2
            let _: serde_json::Value = serde_json::from_str(&contents)?;
            studio_json::validate_json(&contents).map_err(|e| anyhow::anyhow!(e))?;
            eprintln!("JSON is syntactically valid. Schema validation not yet implemented.");
        }
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

    Ok(())
}
