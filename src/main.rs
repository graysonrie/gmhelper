mod sprites;

use clap::{Parser, Subcommand};
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::mpsc;

use crate::aseprite_exporter::{ensure_script_available, export_tags};

mod hot_reloader;
mod aseprite_exporter;

const EXPORT_TAGS_SCRIPT: &str = include_str!("../lua/export_tags.lua");

#[derive(Parser)]
#[command(name = "gmhelper")]
#[command(about = "GameMaker helper tools: sprite watcher & music exporter")]
struct Cli {
    #[command(subcommand)]
    command: SubCmd,
}

#[derive(Subcommand)]
enum SubCmd {
    /// Watch a directory for .aseprite file changes and export tagged frames
    Sprites {
        /// Directory to watch for .aseprite files
        #[arg(short, long, value_name = "DIRECTORY")]
        directory: Option<PathBuf>,

        /// Start watching the current working directory
        #[arg(short, long)]
        start: bool,
    },

    /// Export WAV files from a music/ folder in the cwd as GameMaker-ready OGG files
    Music,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        SubCmd::Sprites { directory, start } => run_sprites(directory, start),
        SubCmd::Music => run_music(),
    }
}

// ---------------------------------------------------------------------------
// Sprites subcommand
// ---------------------------------------------------------------------------

fn run_sprites(directory: Option<PathBuf>, start: bool) {
    let watch_directory = if start {
        std::env::current_dir().unwrap_or_else(|e| {
            eprintln!("Error: Failed to get current directory: {e}");
            std::process::exit(1);
        })
    } else if let Some(dir) = directory {
        dir
    } else {
        std::env::current_dir().unwrap_or_else(|e| {
            eprintln!("Error: Failed to get current directory: {e}");
            eprintln!("Hint: Use --directory <path> or --start to specify a directory");
            std::process::exit(1);
        })
    };

    if !watch_directory.exists() {
        eprintln!(
            "Error: Directory '{}' does not exist",
            watch_directory.display()
        );
        std::process::exit(1);
    }

    if !watch_directory.is_dir() {
        eprintln!("Error: '{}' is not a directory", watch_directory.display());
        std::process::exit(1);
    }

    let script_path = ensure_script_available().unwrap_or_else(|e| {
        eprintln!("Error: Failed to set up export script: {e}");
        std::process::exit(1);
    });

    println!("Watching directory: {}", watch_directory.display());
    println!("Press Ctrl+C to stop...\n");

    let (tx, rx) = mpsc::channel();

    let mut watcher =
        RecommendedWatcher::new(tx, Config::default()).expect("Failed to create file watcher");

    watcher
        .watch(&watch_directory, RecursiveMode::Recursive)
        .expect("Failed to watch directory");

    loop {
        match rx.recv() {
            Ok(Ok(event)) => {
                if let EventKind::Modify(_) | EventKind::Create(_) = event.kind {
                    for path in event.paths {
                        if let Some(ext) = path.extension() {
                            if ext == "aseprite" && path.exists() {
                                println!("Processing: {}", path.display());
                                if let Err(e) = export_tags(&path, &script_path) {
                                    eprintln!("Error exporting {}: {}", path.display(), e);
                                }
                            }
                        }
                    }
                }
            }
            Ok(Err(e)) => eprintln!("Watch error: {e}"),
            Err(e) => eprintln!("Channel error: {e}"),
        }
    }
}

// ---------------------------------------------------------------------------
// Music subcommand
// ---------------------------------------------------------------------------

fn run_music() {
    let cwd = std::env::current_dir().unwrap_or_else(|e| {
        eprintln!("Error: Failed to get current directory: {e}");
        std::process::exit(1);
    });

    println!("Exporting game music from: {}", cwd.display());

    let options = ost_export::GameMusicExportOptions::famitracker_defaults();

    match ost_export::export_as_game_music(&cwd, &options) {
        Ok(result) => println!(
            "Music export complete. Exported {} files.",
            result.num_files_exported
        ),
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
}
