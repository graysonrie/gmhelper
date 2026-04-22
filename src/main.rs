mod history;
mod sprites;

use clap::{Parser, Subcommand};
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use ost_export::Mp4ExportOptions;
use std::path::PathBuf;
use std::sync::mpsc;

use crate::aseprite_exporter::{ensure_script_available, export_tags};

mod aseprite_exporter;
mod code_editor;
mod hot_reloader;

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

        /// Path to a GameMaker .yyp project file. When set, exported frames are
        /// imported directly into the project instead of being saved as GIF/PNG.
        #[arg(short, long, value_name = "YYP_FILE")]
        project: Option<PathBuf>,
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

    /// List recent gmhelper invocations, or re-run one by number (#1 = most recent)
    Previous {
        /// Re-execute the Nth most recent command (1–10; 1 = most recent)
        #[arg(value_name = "N", value_parser = clap::value_parser!(u8).range(1..=10))]
        index: Option<u8>,
    },
}

fn main() {
    let cli = Cli::parse();

    if !matches!(&cli.command, SubCmd::Previous { .. })
        && let Err(e) = history::record_current_invocation()
    {
        eprintln!("Warning: could not save command history: {e}");
    }

    match cli.command {
        SubCmd::Sprites {
            directory,
            start,
            project,
        } => run_sprites(directory, start, project),
        SubCmd::Music {
            mp4,
            game_name,
            image_path,
        } => run_music(mp4, game_name, image_path),
        SubCmd::Reload { project } => hot_reloader::run_reload(project),
        SubCmd::Previous { index: None } => {
            let h = history::load();
            print!("{}", history::list_text(&h));
        }
        SubCmd::Previous { index: Some(n) } => match history::reexecute(n) {
            Ok(status) => std::process::exit(status.code().unwrap_or(1)),
            Err(e) => {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        },
    }
}

// ---------------------------------------------------------------------------
// Sprites subcommand
// ---------------------------------------------------------------------------

fn run_sprites(directory: Option<PathBuf>, start: bool, project: Option<PathBuf>) {
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

    let project_path = project.inspect(|p| {
        if !p.exists() {
            eprintln!("Error: Project file '{}' does not exist", p.display());
            std::process::exit(1);
        }
        match p.extension().and_then(|e| e.to_str()) {
            Some("yyp") => {}
            _ => {
                eprintln!(
                    "Error: '{}' is not a .yyp file. Provide a valid GameMaker project file.",
                    p.display()
                );
                std::process::exit(1);
            }
        }
    });

    let script_path = ensure_script_available().unwrap_or_else(|e| {
        eprintln!("Error: Failed to set up export script: {e}");
        std::process::exit(1);
    });

    println!("Watching directory: {}", watch_directory.display());
    if let Some(ref pp) = project_path {
        println!("GameMaker project: {}", pp.display());
    }

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
                            if let Err(e) = export_tags(
                                &path,
                                &script_path,
                                project_path.as_deref(),
                                &watch_directory,
                            ) {
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
