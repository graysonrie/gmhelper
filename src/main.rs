mod gm_config;
mod history;
mod sprites;

use clap::{Parser, Subcommand};
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use ost_export::Mp4ExportOptions;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::{Duration, Instant};

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

    Config {
        #[arg(short, long)]
        beta: Option<bool>,

        /// Path to a GameMaker .yyp project file. When set, exported frames are
        /// imported directly into the project instead of being saved as GIF/PNG.
        #[arg(short, long, value_name = "YYP_FILE")]
        all_sprites_export_project: Option<String>,
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

    AllSprites,
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
        SubCmd::Config {
            beta,
            all_sprites_export_project,
        } => run_config(beta, all_sprites_export_project),
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
        SubCmd::AllSprites => run_all_sprites(),
    }
}

// ---------------------------------------------------------------------------
// All Sprites subcommand
// ---------------------------------------------------------------------------
fn run_all_sprites() {
    let path_to_sprites_dir = std::env::current_dir().unwrap_or_else(|e| {
        eprintln!("Error: Failed to get current directory: {e}");
        std::process::exit(1);
    });

    let config = gm_config::get_or_create_config().unwrap_or_else(|e| {
        eprintln!("Error: Failed to get config: {e}");
        std::process::exit(1);
    });

    let project_path = config.all_sprites_export_yyp_path.unwrap_or_else(|| {
        eprintln!("Error: All Sprites export path is not set in the config");
        std::process::exit(1);
    });

    aseprite_exporter::export_all_sprites(&path_to_sprites_dir, Path::new(&project_path), true)
        .unwrap_or_else(|e| {
            eprintln!("Error: Failed to get current directory: {e}");
            std::process::exit(1);
        });

    println!("All sprites exported successfully");
}

// ---------------------------------------------------------------------------
// Sprites subcommand
// ---------------------------------------------------------------------------

const SPRITE_DEBOUNCE: Duration = Duration::from_millis(500);
const SPRITE_POLL_INTERVAL: Duration = Duration::from_millis(100);

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

    let mut pending_exports: HashMap<PathBuf, Instant> = HashMap::new();
    let mut in_flight: HashSet<PathBuf> = HashSet::new();

    loop {
        match rx.recv_timeout(SPRITE_POLL_INTERVAL) {
            Ok(Ok(event)) => {
                if let EventKind::Modify(_) | EventKind::Create(_) = event.kind {
                    let mut seen = HashSet::new();
                    for path in event.paths {
                        if seen.insert(path.clone())
                            && path.extension().and_then(|e| e.to_str()) == Some("aseprite")
                            && path.exists()
                        {
                            pending_exports.insert(path, Instant::now());
                        }
                    }
                }
            }
            Ok(Err(e)) => eprintln!("Watch error: {e}"),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }

        let ready: Vec<PathBuf> = pending_exports
            .iter()
            .filter(|(path, last_change)| {
                !in_flight.contains(*path) && last_change.elapsed() >= SPRITE_DEBOUNCE
            })
            .map(|(path, _)| path.clone())
            .collect();

        for path in ready {
            pending_exports.remove(&path);
            in_flight.insert(path.clone());

            println!("Processing: {}", path.display());
            if let Err(e) = export_tags(
                &path,
                &script_path,
                project_path.as_deref(),
                &watch_directory,
            ) {
                eprintln!("Error exporting {}: {}", path.display(), e);
            }

            in_flight.remove(&path);
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

pub fn run_config(beta: Option<bool>, all_sprites_export_project: Option<String>) {
    // Only proceed if at least one value is Some(_)
    if beta.is_some() || all_sprites_export_project.is_some() {
        match gm_config::get_or_create_config() {
            Ok(mut config) => {
                if let Some(beta) = beta {
                    if beta {
                        println!("Setting config to use GM Beta");
                    } else {
                        println!("Setting config to use default GM");
                    }
                    config.use_gm_beta = beta;
                }
                if let Some(project) = all_sprites_export_project {
                    println!("Setting all_sprites_export_yyp_path to '{project}'");
                    config.all_sprites_export_yyp_path = Some(project);
                }
                if let Err(e) = gm_config::write_config(&config) {
                    eprintln!("Error writing config: {e}");
                    std::process::exit(1);
                }
            }
            Err(e) => {
                eprintln!("Could not get config: {e}");
                std::process::exit(1);
            }
        }
    }
}
