mod sprites;

use clap::Parser;
use image::DynamicImage;
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;

// Embed the Lua script in the binary
const EXPORT_TAGS_SCRIPT: &str = include_str!("../lua/export_tags.lua");

#[derive(Parser)]
#[command(name = "gmhelper")]
#[command(about = "Watches a directory for .aseprite file changes and exports tagged frames", long_about = None)]
struct Args {
    /// Directory to watch for .aseprite files
    #[arg(short, long, value_name = "DIRECTORY")]
    directory: Option<PathBuf>,

    /// Start watching the current working directory
    #[arg(short, long)]
    start: bool,
}

fn main() {
    let args = Args::parse();

    // Determine which directory to watch
    let watch_directory = if args.start {
        // Use current working directory
        std::env::current_dir().unwrap_or_else(|e| {
            eprintln!("Error: Failed to get current directory: {e}");
            std::process::exit(1);
        })
    } else if let Some(dir) = args.directory {
        // Use specified directory
        dir
    } else {
        // Default to current working directory if neither flag is provided
        std::env::current_dir().unwrap_or_else(|e| {
            eprintln!("Error: Failed to get current directory: {e}");
            eprintln!("Hint: Use --directory <path> or --start to specify a directory");
            std::process::exit(1);
        })
    };

    // Verify directory exists
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

    // Ensure the export script is available and get its path
    let script_path = ensure_script_available().unwrap_or_else(|e| {
        eprintln!("Error: Failed to set up export script: {e}");
        std::process::exit(1);
    });

    println!("Watching directory: {}", watch_directory.display());
    println!("Press Ctrl+C to stop...\n");

    // Create a channel to receive events
    let (tx, rx) = mpsc::channel();

    // Create a watcher object
    let mut watcher =
        RecommendedWatcher::new(tx, Config::default()).expect("Failed to create file watcher");

    // Watch the directory recursively
    watcher
        .watch(&watch_directory, RecursiveMode::Recursive)
        .expect("Failed to watch directory");

    // Process events
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

#[derive(Debug, Deserialize)]
struct SpriteExportInfo {
    path: String,
    width: u32,
    height: u32,
    frame_count: u32,
    tag_name: String,
}

fn export_tags(aseprite_path: &Path, script_path: &Path) -> Result<(), String> {
    // Get the output directory (same as the .aseprite file)
    let output_dir = aseprite_path
        .parent()
        .ok_or_else(|| "Could not get parent directory".to_string())?;

    let file_path_str = aseprite_path.to_str().ok_or("Invalid file path")?;

    let output_dir_str = output_dir.to_str().ok_or("Invalid output directory path")?;

    let script_path_str = script_path.to_str().ok_or("Invalid script path")?;

    // Invoke Aseprite with the export script
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

    // Print stdout (Aseprite's JSON output)
    if !output.stdout.is_empty() {
        print!("{}", String::from_utf8_lossy(&output.stdout));
    }

    // Parse JSON_EXPORT lines from stderr (where Lua outputs them)
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
            // Print other stderr output (but not empty lines)
            eprintln!("{line}");
        }
    }

    if !output.status.success() {
        return Err(format!(
            "Aseprite exited with code: {}",
            output.status.code().unwrap_or(-1)
        ));
    }

    // Process each exported spritesheet
    if export_infos.is_empty() {
        eprintln!(
            "Warning: No export info received from Lua script. Check if JSON_EXPORT lines are being output."
        );
    } else {
        println!("Found {} spritesheet(s) to process", export_infos.len());
    }

    for info in export_infos {
        println!("Processing spritesheet: {}", info.path);
        if let Err(e) = split_spritesheet(&info, output_dir) {
            eprintln!("Error splitting spritesheet {}: {e}", info.path);
        }
    }

    Ok(())
}

fn split_spritesheet(info: &SpriteExportInfo, output_dir: &Path) -> Result<(), String> {
    let spritesheet_path = Path::new(&info.path);

    if !spritesheet_path.exists() {
        return Err(format!("Spritesheet not found: {}", info.path));
    }

    // Load the spritesheet image (ensure we preserve alpha channel)
    let img = image::open(spritesheet_path)
        .map_err(|e| format!("Failed to load spritesheet: {e}"))?
        .into_rgba8();

    // Convert back to DynamicImage to maintain alpha
    let img = DynamicImage::ImageRgba8(img);

    let sheet_width = img.width();
    let sheet_height = img.height();
    let frame_width = info.width;
    let frame_height = info.height;
    let frame_count = info.frame_count as usize;

    // Calculate how many frames fit horizontally
    let frames_per_row = (sheet_width / frame_width) as usize;
    let num_rows = (sheet_height / frame_height) as usize;

    // Extract individual frames
    let mut frames = Vec::new();
    for row in 0..num_rows {
        for col in 0..frames_per_row {
            if frames.len() >= frame_count {
                break;
            }

            let x = col as u32 * frame_width;
            let y = row as u32 * frame_height;

            // Crop the frame from the spritesheet
            let frame = img.crop_imm(x, y, frame_width, frame_height);
            frames.push(frame);
        }
        if frames.len() >= frame_count {
            break;
        }
    }

    // Determine output filename (GIF for multiple frames, PNG for single)
    let base_name = spritesheet_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or("Invalid spritesheet filename")?;

    // Ensure we have at least one frame
    if frames.is_empty() {
        return Err("No frames extracted from spritesheet".to_string());
    }

    let output_path = if frames.len() > 1 {
        // Create animated GIF
        let gif_path = output_dir.join(format!("{base_name}.gif"));
        create_gif(&frames, &gif_path, frame_width, frame_height)?;
        gif_path
    } else {
        // Save as PNG (preserve alpha channel)
        let png_path = output_dir.join(format!("{base_name}.png"));
        // Ensure we save with alpha channel preserved
        let rgba_frame = frames[0].to_rgba8();
        rgba_frame
            .save(&png_path)
            .map_err(|e| format!("Failed to save PNG: {e}"))?;
        png_path
    };

    // Remove the temporary spritesheet only if it's different from the output file
    // (When there's only 1 frame, the spritesheet IS the output file, so don't delete it)
    // Normalize paths for comparison (handle different separators, etc.)
    let spritesheet_canonical = spritesheet_path.canonicalize().ok();
    let output_canonical = output_path.canonicalize().ok();

    // Only delete if paths are different (after canonicalization if possible)
    let should_delete =
        if let (Some(sheet), Some(out)) = (&spritesheet_canonical, &output_canonical) {
            sheet != out
        } else {
            // If canonicalization failed, compare the original paths
            spritesheet_path != output_path
        };

    if should_delete {
        fs::remove_file(spritesheet_path)
            .map_err(|e| format!("Failed to remove spritesheet: {e}"))?;
    }

    println!(
        "Created: {} ({} frame{})",
        output_path.display(),
        frame_count,
        if frame_count > 1 { "s" } else { "" }
    );

    Ok(())
}

// Helper function to find the nearest color in the palette using Euclidean distance
fn find_nearest_color(color: [u8; 3], palette: &[[u8; 3]]) -> usize {
    // If palette only has transparent marker (pure black), return it (shouldn't happen in practice)
    if palette.len() <= 1 {
        return 0;
    }

    let mut best_idx = 1; // Start at 1 to skip index 0 (transparent marker - pure black)
    let mut best_dist = u32::MAX;

    for (idx, &palette_color) in palette.iter().enumerate().skip(1) {
        // Skip index 0 (transparent marker - pure black)
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
    // Convert u32 to u16 for GIF encoder (GIF format limitation)
    let width_u16 = width
        .try_into()
        .map_err(|_| format!("Width {width} exceeds GIF limit (65535)"))?;
    let height_u16 = height
        .try_into()
        .map_err(|_| format!("Height {height} exceeds GIF limit (65535)"))?;

    let mut file = std::fs::File::create(output_path)
        .map_err(|e| format!("Failed to create GIF file: {e}"))?;

    // Build a custom palette with transparent color at index 0
    // Use pure black [0, 0, 0] as the transparent marker
    // This means pure black cannot be used as an opaque color in sprites
    let transparent_marker = [0u8, 0u8, 0u8];

    // Collect all unique opaque colors from all frames
    let mut color_map = std::collections::HashMap::new();
    let mut color_list = vec![transparent_marker]; // Index 0 is transparent marker (pure black)

    // First pass: collect all unique colors
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
                // Skip pure black (transparent marker) - it cannot be used as an opaque color
                // This allows blue/cyan colors to work properly
                if color != transparent_marker && !color_map.contains_key(&color) {
                    color_map.insert(color, color_list.len());
                    color_list.push(color);
                }
            }
        }
    }

    // Build palette (RGB triplets)
    let mut palette = Vec::new();
    for color in &color_list {
        palette.push(color[0]);
        palette.push(color[1]);
        palette.push(color[2]);
    }

    // Limit to 256 colors (GIF limitation)
    if palette.len() > 768 {
        palette.truncate(768);
        color_list.truncate(256);
        // Rebuild color_map with truncated colors
        color_map.clear();
        for (idx, color) in color_list.iter().enumerate() {
            color_map.insert(*color, idx);
        }
    }

    // Build palette array for nearest color lookup (skip index 0)
    let palette_colors: Vec<[u8; 3]> = color_list.clone();

    let mut encoder = gif::Encoder::new(&mut file, width_u16, height_u16, &palette)
        .map_err(|e| format!("Failed to create GIF encoder: {e}"))?;

    // Set repeat to infinite
    encoder
        .set_repeat(gif::Repeat::Infinite)
        .map_err(|e| format!("Failed to set GIF repeat: {e}"))?;

    // Process frames and convert to palette indices
    for frame_img in frames {
        let rgba_img = frame_img.to_rgba8();
        let pixels = rgba_img.as_raw();

        // Convert to palette indices
        let mut indexed_pixels = Vec::new();
        let mut has_transparent = false;

        for chunk in pixels.chunks(4) {
            let r = chunk[0];
            let g = chunk[1];
            let b = chunk[2];
            let a = chunk[3];

            if a == 0 {
                // Transparent pixel - use index 0 (transparent marker)
                indexed_pixels.push(0);
                has_transparent = true;
            } else {
                // Opaque pixel - find color in palette
                let color = [r, g, b];
                let index = color_map.get(&color).copied().unwrap_or_else(|| {
                    // If color not in palette, find nearest color instead of using transparent marker
                    find_nearest_color(color, &palette_colors)
                });
                indexed_pixels.push(index as u8);
            }
        }

        // Convert indexed pixels back to RGB for the frame
        // Index 0 will be converted to the transparent marker color, which will be marked as transparent
        let mut rgb_for_frame = Vec::new();
        for &idx in &indexed_pixels {
            let color_idx = idx as usize * 3;
            if color_idx + 2 < palette.len() {
                rgb_for_frame.push(palette[color_idx]);
                rgb_for_frame.push(palette[color_idx + 1]);
                rgb_for_frame.push(palette[color_idx + 2]);
            } else {
                // Fallback to transparent marker (shouldn't happen with proper palette)
                rgb_for_frame.push(transparent_marker[0]);
                rgb_for_frame.push(transparent_marker[1]);
                rgb_for_frame.push(transparent_marker[2]);
            }
        }

        let mut frame = gif::Frame::from_rgb(width_u16, height_u16, &rgb_for_frame);
        frame.delay = 10; // 100ms delay
        frame.dispose = gif::DisposalMethod::Background;
        frame.left = 0;
        frame.top = 0;

        // Set transparent color to index 0 (our transparent marker)
        // This tells the GIF encoder that pixels with the color at index 0 should be transparent
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
    // First, try to find an existing script in the project directory (for development)
    let dev_script = Path::new("lua/export_tags.lua");
    if dev_script.exists() {
        return Ok(dev_script.to_path_buf());
    }

    // Get the executable directory
    let exe_path =
        std::env::current_exe().map_err(|e| format!("Failed to get executable path: {e}"))?;
    let exe_dir = exe_path
        .parent()
        .ok_or_else(|| "Could not get executable directory".to_string())?;

    // Create a scripts directory next to the executable
    let scripts_dir = exe_dir.join("lua");
    let script_path = scripts_dir.join("export_tags.lua");

    // If the script already exists and matches, use it
    if script_path.exists() {
        // Optionally verify the content matches (for updates)
        if let Ok(existing_content) = fs::read_to_string(&script_path) {
            if existing_content == EXPORT_TAGS_SCRIPT {
                return Ok(script_path);
            }
        }
    }

    // Create the scripts directory if it doesn't exist
    fs::create_dir_all(&scripts_dir).map_err(|e| {
        format!(
            "Failed to create scripts directory at {}: {e}",
            scripts_dir.display()
        )
    })?;

    // Write the embedded script to the file
    fs::write(&script_path, EXPORT_TAGS_SCRIPT)
        .map_err(|e| format!("Failed to write script to {}: {e}", script_path.display()))?;

    Ok(script_path)
}
