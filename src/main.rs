mod sprites;

use clap::{Parser, Subcommand};
use image::DynamicImage;
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;

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
    Music,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        SubCmd::Sprites {
            directory,
            start,
            project,
        } => run_sprites(directory, start, project),
        SubCmd::Music => run_music(),
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
                        if let Some(ext) = path.extension() {
                            if ext == "aseprite" && path.exists() {
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

// ---------------------------------------------------------------------------
// Sprite export internals (unchanged)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct SpriteExportInfo {
    path: String,
    width: u32,
    height: u32,
    frame_count: u32,
    tag_name: String,
}

fn export_tags(
    aseprite_path: &Path,
    script_path: &Path,
    project_path: Option<&Path>,
    watch_dir: &Path,
) -> Result<(), String> {
    let output_dir = aseprite_path
        .parent()
        .ok_or_else(|| "Could not get parent directory".to_string())?;

    let file_path_str = aseprite_path.to_str().ok_or("Invalid file path")?;
    let output_dir_str = output_dir.to_str().ok_or("Invalid output directory path")?;
    let script_path_str = script_path.to_str().ok_or("Invalid script path")?;

    let output = Command::new("aseprite")
        .arg("-b")
        .arg("-script-param")
        .arg(format!("filepath={file_path_str}"))
        .arg("-script-param")
        .arg(format!("outputdir={output_dir_str}"))
        .arg("-script")
        .arg(script_path_str)
        .output()
        .map_err(|e| {
            format!("Failed to execute Aseprite: {e}. Make sure 'aseprite' is in your PATH.")
        })?;

    if !output.stdout.is_empty() {
        print!("{}", String::from_utf8_lossy(&output.stdout));
    }

    let stderr_str = String::from_utf8_lossy(&output.stderr);
    let mut export_infos = Vec::new();

    for line in stderr_str.lines() {
        if line.starts_with("JSON_EXPORT:") {
            let json_str = line.strip_prefix("JSON_EXPORT:").unwrap();
            match serde_json::from_str::<SpriteExportInfo>(json_str) {
                Ok(info) => export_infos.push(info),
                Err(e) => {
                    eprintln!("Warning: Failed to parse export info: {e}");
                    eprintln!("JSON string was: {json_str}");
                }
            }
        } else if !line.trim().is_empty() {
            eprintln!("{line}");
        }
    }

    if !output.status.success() {
        return Err(format!(
            "Aseprite exited with code: {}",
            output.status.code().unwrap_or(-1)
        ));
    }

    if export_infos.is_empty() {
        eprintln!(
            "Warning: No export info received from Lua script. Check if JSON_EXPORT lines are being output."
        );
    } else {
        println!("Found {} spritesheet(s) to process", export_infos.len());
    }

    for info in &export_infos {
        println!("Processing spritesheet: {}", info.path);

        let frames = match extract_frames(info) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("Error extracting frames from {}: {e}", info.path);
                continue;
            }
        };

        if let Some(yyp) = project_path {
            let sprite_name =
                sprites::gm_import::derive_sprite_name(aseprite_path, &info.tag_name)?;
            let gm_folder = sprites::gm_import::compute_gm_folder_path(watch_dir, aseprite_path);

            if let Err(e) = sprites::gm_import::import_sprite_to_project(
                yyp,
                &sprite_name,
                &frames,
                &gm_folder,
                info.width,
                info.height,
            ) {
                eprintln!("Error importing sprite to GM project: {e}");
            }
        } else if let Err(e) = save_frames_as_output(info, &frames, output_dir) {
            eprintln!("Error saving output for {}: {e}", info.path);
        }

        let spritesheet_path = Path::new(&info.path);
        if spritesheet_path.exists() {
            if let Err(e) = fs::remove_file(spritesheet_path) {
                eprintln!("Warning: Failed to remove temporary spritesheet: {e}");
            }
        }
    }

    Ok(())
}

fn extract_frames(info: &SpriteExportInfo) -> Result<Vec<DynamicImage>, String> {
    let spritesheet_path = Path::new(&info.path);

    if !spritesheet_path.exists() {
        return Err(format!("Spritesheet not found: {}", info.path));
    }

    let img = image::open(spritesheet_path)
        .map_err(|e| format!("Failed to load spritesheet: {e}"))?
        .into_rgba8();
    let img = DynamicImage::ImageRgba8(img);

    let sheet_width = img.width();
    let sheet_height = img.height();
    let frame_width = info.width;
    let frame_height = info.height;
    let frame_count = info.frame_count as usize;

    let frames_per_row = (sheet_width / frame_width) as usize;
    let num_rows = (sheet_height / frame_height) as usize;

    let mut frames = Vec::new();
    for row in 0..num_rows {
        for col in 0..frames_per_row {
            if frames.len() >= frame_count {
                break;
            }
            let x = col as u32 * frame_width;
            let y = row as u32 * frame_height;
            let frame = img.crop_imm(x, y, frame_width, frame_height);
            frames.push(frame);
        }
        if frames.len() >= frame_count {
            break;
        }
    }

    if frames.is_empty() {
        return Err("No frames extracted from spritesheet".to_string());
    }

    Ok(frames)
}

fn save_frames_as_output(
    info: &SpriteExportInfo,
    frames: &[DynamicImage],
    output_dir: &Path,
) -> Result<(), String> {
    let spritesheet_path = Path::new(&info.path);
    let base_name = spritesheet_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or("Invalid spritesheet filename")?;

    let output_path = if frames.len() > 1 {
        let gif_path = output_dir.join(format!("{base_name}.gif"));
        create_gif(frames, &gif_path, info.width, info.height)?;
        gif_path
    } else {
        let png_path = output_dir.join(format!("{base_name}.png"));
        let rgba_frame = frames[0].to_rgba8();
        rgba_frame
            .save(&png_path)
            .map_err(|e| format!("Failed to save PNG: {e}"))?;
        png_path
    };

    println!(
        "Created: {} ({} frame{})",
        output_path.display(),
        frames.len(),
        if frames.len() > 1 { "s" } else { "" }
    );

    Ok(())
}

fn find_nearest_color(color: [u8; 3], palette: &[[u8; 3]]) -> usize {
    if palette.len() <= 1 {
        return 0;
    }

    let mut best_idx = 1;
    let mut best_dist = u32::MAX;

    for (idx, &palette_color) in palette.iter().enumerate().skip(1) {
        let dr = color[0] as i32 - palette_color[0] as i32;
        let dg = color[1] as i32 - palette_color[1] as i32;
        let db = color[2] as i32 - palette_color[2] as i32;
        let dist = (dr * dr + dg * dg + db * db) as u32;

        if dist < best_dist {
            best_dist = dist;
            best_idx = idx;
        }
    }

    best_idx
}

fn create_gif(
    frames: &[DynamicImage],
    output_path: &Path,
    width: u32,
    height: u32,
) -> Result<(), String> {
    let width_u16 = width
        .try_into()
        .map_err(|_| format!("Width {width} exceeds GIF limit (65535)"))?;
    let height_u16 = height
        .try_into()
        .map_err(|_| format!("Height {height} exceeds GIF limit (65535)"))?;

    let mut file = std::fs::File::create(output_path)
        .map_err(|e| format!("Failed to create GIF file: {e}"))?;

    let transparent_marker = [0u8, 0u8, 0u8];

    let mut color_map = std::collections::HashMap::new();
    let mut color_list = vec![transparent_marker];

    for frame_img in frames {
        let rgba_img = frame_img.to_rgba8();
        let pixels = rgba_img.as_raw();
        for chunk in pixels.chunks(4) {
            let r = chunk[0];
            let g = chunk[1];
            let b = chunk[2];
            let a = chunk[3];

            if a > 0 {
                let color = [r, g, b];
                if color != transparent_marker && !color_map.contains_key(&color) {
                    color_map.insert(color, color_list.len());
                    color_list.push(color);
                }
            }
        }
    }

    let mut palette = Vec::new();
    for color in &color_list {
        palette.push(color[0]);
        palette.push(color[1]);
        palette.push(color[2]);
    }

    if palette.len() > 768 {
        palette.truncate(768);
        color_list.truncate(256);
        color_map.clear();
        for (idx, color) in color_list.iter().enumerate() {
            color_map.insert(*color, idx);
        }
    }

    let palette_colors: Vec<[u8; 3]> = color_list.clone();

    let mut encoder = gif::Encoder::new(&mut file, width_u16, height_u16, &palette)
        .map_err(|e| format!("Failed to create GIF encoder: {e}"))?;

    encoder
        .set_repeat(gif::Repeat::Infinite)
        .map_err(|e| format!("Failed to set GIF repeat: {e}"))?;

    for frame_img in frames {
        let rgba_img = frame_img.to_rgba8();
        let pixels = rgba_img.as_raw();

        let mut indexed_pixels = Vec::new();
        let mut has_transparent = false;

        for chunk in pixels.chunks(4) {
            let r = chunk[0];
            let g = chunk[1];
            let b = chunk[2];
            let a = chunk[3];

            if a == 0 {
                indexed_pixels.push(0);
                has_transparent = true;
            } else {
                let color = [r, g, b];
                let index = color_map
                    .get(&color)
                    .copied()
                    .unwrap_or_else(|| find_nearest_color(color, &palette_colors));
                indexed_pixels.push(index as u8);
            }
        }

        let mut rgb_for_frame = Vec::new();
        for &idx in &indexed_pixels {
            let color_idx = idx as usize * 3;
            if color_idx + 2 < palette.len() {
                rgb_for_frame.push(palette[color_idx]);
                rgb_for_frame.push(palette[color_idx + 1]);
                rgb_for_frame.push(palette[color_idx + 2]);
            } else {
                rgb_for_frame.push(transparent_marker[0]);
                rgb_for_frame.push(transparent_marker[1]);
                rgb_for_frame.push(transparent_marker[2]);
            }
        }

        let mut frame = gif::Frame::from_rgb(width_u16, height_u16, &rgb_for_frame);
        frame.delay = 10;
        frame.dispose = gif::DisposalMethod::Background;
        frame.left = 0;
        frame.top = 0;

        if has_transparent {
            frame.transparent = Some(0);
        }

        encoder
            .write_frame(&frame)
            .map_err(|e| format!("Failed to write GIF frame: {e}"))?;
    }

    Ok(())
}

fn ensure_script_available() -> Result<PathBuf, String> {
    let dev_script = Path::new("lua/export_tags.lua");
    if dev_script.exists() {
        return Ok(dev_script.to_path_buf());
    }

    let exe_path =
        std::env::current_exe().map_err(|e| format!("Failed to get executable path: {e}"))?;
    let exe_dir = exe_path
        .parent()
        .ok_or_else(|| "Could not get executable directory".to_string())?;

    let scripts_dir = exe_dir.join("lua");
    let script_path = scripts_dir.join("export_tags.lua");

    if script_path.exists() {
        if let Ok(existing_content) = fs::read_to_string(&script_path) {
            if existing_content == EXPORT_TAGS_SCRIPT {
                return Ok(script_path);
            }
        }
    }

    fs::create_dir_all(&scripts_dir).map_err(|e| {
        format!(
            "Failed to create scripts directory at {}: {e}",
            scripts_dir.display()
        )
    })?;

    fs::write(&script_path, EXPORT_TAGS_SCRIPT)
        .map_err(|e| format!("Failed to write script to {}: {e}", script_path.display()))?;

    Ok(script_path)
}
