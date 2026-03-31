mod sprites;

use clap::{Parser, Subcommand};
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use ost_export::Mp4ExportOptions;
use std::path::PathBuf;
use std::sync::mpsc;

use crate::aseprite_exporter::{ensure_script_available, export_tags};

mod aseprite_exporter;
mod hot_reloader;
mod code_editor;

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
    Music {
        #[arg(short, long)]
        mp4: bool,

        #[arg(short, long, value_name = "GAME_NAME")]
        game_name: Option<String>,

        #[arg(short, long, value_name = "IMAGE_PATH")]
        image_path: Option<String>,
    },

    /// Hot-reload: watch .gml files and rebuild + relaunch the game on changes
    Reload {
        /// Path to the GameMaker .yyp project file
        #[arg(value_name = "YYP_FILE")]
        project: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        SubCmd::Sprites { directory, start } => run_sprites(directory, start),
        SubCmd::Music {
            mp4,
            game_name,
            image_path,
        } => run_music(mp4, game_name, image_path),
        SubCmd::Reload { project } => hot_reloader::run_reload(project),
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
                        if let Some(ext) = path.extension()
                            && ext == "aseprite"
                            && path.exists()
                        {
                            println!("Processing: {}", path.display());
                            if let Err(e) = export_tags(&path, &script_path) {
                                eprintln!("Error exporting {}: {}", path.display(), e);
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

fn run_music(mp4: bool, game_name: Option<String>, image_path: Option<String>) {
    let cwd = std::env::current_dir().unwrap_or_else(|e| {
        eprintln!("Error: Failed to get current directory: {e}");
        std::process::exit(1);
    });

    let options = ost_export::GameMusicExportOptions::famitracker_defaults();
    if mp4 {
        println!("Exporting game music from: {} as MP4 files", cwd.display());

        let game_title = game_name.expect("You must provide a game_name if exporting mp4");
        let video_image_path = image_path.expect("You must provide a image_path if exporting mp4");
        let mp4_options = Mp4ExportOptions::defaults(&video_image_path, &game_title);

        match ost_export::export_as_mp4_files(&cwd, &options, &mp4_options) {
            Ok(result) => println!(
                "MP4 export complete. Exported {} files.",
                result.num_files_exported
            ),
            Err(e) => {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }
    } else {
        println!("Exporting game music from: {}", cwd.display());

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
}
