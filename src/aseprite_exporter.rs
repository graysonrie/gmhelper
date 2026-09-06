use image::DynamicImage;
use rayon::prelude::*;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use walkdir::WalkDir;

use crate::{EXPORT_TAGS_SCRIPT, export_cache, sprites};

// ---------------------------------------------------------------------------
// Sprite export internals
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct SpriteExportInfo {
    path: String,
    width: u32,
    height: u32,
    frame_count: u32,
    tag_name: String,
}

pub struct PreparedSpriteImport {
    pub aseprite_path: PathBuf,
    pub tag_name: String,
    #[allow(unused)]
    pub spritesheet_stem: String,
    pub frames: Vec<DynamicImage>,
    pub width: u32,
    pub height: u32,
    pub gm_folder: String,
}

const ASEPRITE_EXPORT_THREADS: usize = 4;

#[allow(unused)]
pub fn export_tags(
    aseprite_path: &Path,
    script_path: &Path,
    project_path: Option<&Path>,
    watch_dir: &Path,
) -> Result<(), String> {
    let output_dir = aseprite_path
        .parent()
        .ok_or_else(|| "Could not get parent directory".to_string())?;

    let prepared = prepare_aseprite_export(aseprite_path, script_path, watch_dir)?;

    for import in prepared {
        if let Some(yyp) = project_path {
            let sprite_name =
                sprites::gm_import::derive_sprite_name(aseprite_path, &import.tag_name)?;

            if let Err(e) = sprites::gm_import::import_sprite_to_project(
                yyp,
                &sprite_name,
                &import.frames,
                &import.gm_folder,
                import.width,
                import.height,
            ) {
                eprintln!("Error importing sprite to GM project: {e}");
            }
        } else if let Err(e) = save_frames_as_output_from_tag(
            &import.spritesheet_stem,
            &import.frames,
            output_dir,
            import.width,
            import.height,
        ) {
            eprintln!("Error saving output for {}: {e}", import.tag_name);
        }
    }

    Ok(())
}

fn prepare_aseprite_export(
    aseprite_path: &Path,
    script_path: &Path,
    watch_dir: &Path,
) -> Result<Vec<PreparedSpriteImport>, String> {
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

    let gm_folder = sprites::gm_import::compute_gm_folder_path(watch_dir, aseprite_path);
    let mut prepared = Vec::new();
    let mut spritesheets_to_delete = HashSet::new();

    for info in &export_infos {
        println!("Processing spritesheet: {}", info.path);

        let frames = match extract_frames(info) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("Error extracting frames from {}: {e}", info.path);
                continue;
            }
        };

        if frames.len() > 1 {
            spritesheets_to_delete.insert(info.path.clone());
        }

        let spritesheet_stem = Path::new(&info.path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("spritesheet")
            .to_string();

        prepared.push(PreparedSpriteImport {
            aseprite_path: aseprite_path.to_path_buf(),
            tag_name: info.tag_name.clone(),
            spritesheet_stem,
            frames,
            width: info.width,
            height: info.height,
            gm_folder: gm_folder.clone(),
        });
    }

    for path in spritesheets_to_delete {
        let spritesheet_path = Path::new(&path);
        if spritesheet_path.exists()
            && let Err(e) = fs::remove_file(spritesheet_path)
        {
            eprintln!("Warning: Failed to remove temporary spritesheet: {e}");
        }
    }

    Ok(prepared)
}

fn extract_frames(info: &SpriteExportInfo) -> Result<Vec<DynamicImage>, String> {
    let spritesheet_path = Path::new(&info.path);
    const MAX_ATTEMPTS: u32 = 5;
    const RETRY_DELAY: Duration = Duration::from_millis(100);

    let mut last_err = String::new();
    let rgba = 'load: {
        for attempt in 0..MAX_ATTEMPTS {
            if !spritesheet_path.exists() {
                last_err = format!("Spritesheet not found: {}", info.path);
            } else {
                match image::open(spritesheet_path) {
                    Ok(img) => break 'load img.into_rgba8(),
                    Err(e) => last_err = format!("Failed to load spritesheet: {e}"),
                }
            }

            if attempt + 1 < MAX_ATTEMPTS {
                std::thread::sleep(RETRY_DELAY);
            }
        }
        return Err(last_err);
    };

    let img = DynamicImage::ImageRgba8(rgba);

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

fn save_frames_as_output_from_tag(
    base_name: &str,
    frames: &[DynamicImage],
    output_dir: &Path,
    width: u32,
    height: u32,
) -> Result<(), String> {
    let output_path = if frames.len() > 1 {
        let gif_path = output_dir.join(format!("{base_name}.gif"));
        create_gif(frames, &gif_path, width, height)?;
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

pub fn ensure_script_available() -> Result<PathBuf, String> {
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

    if script_path.exists()
        && let Ok(existing_content) = fs::read_to_string(&script_path)
        && existing_content == EXPORT_TAGS_SCRIPT
    {
        return Ok(script_path);
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

pub fn export_all_sprites(
    path_to_sprites_dir: &Path,
    project_path: &Path,
    should_focus_gamemaker: bool,
    force_export: bool,
) -> Result<(), anyhow::Error> {
    let script_path = ensure_script_available().map_err(|e| anyhow::anyhow!(e))?;

    let aseprite_paths: Vec<PathBuf> = WalkDir::new(path_to_sprites_dir)
        .into_iter()
        .flatten()
        .filter(|entry| {
            entry.path().is_file()
                && entry
                    .path()
                    .extension()
                    .is_some_and(|ext| ext == "aseprite")
        })
        .map(|entry| entry.into_path())
        .collect();

    if force_export {
        println!("Cache bypassed (GMHELPER_FORCE_EXPORT)");
    }

    let mut cache = export_cache::ExportCache::load(path_to_sprites_dir, project_path)
        .map_err(anyhow::Error::msg)?;

    let current_rel_paths: HashSet<String> = aseprite_paths
        .iter()
        .map(|path| export_cache::relative_path_key(path_to_sprites_dir, path))
        .collect::<Result<_, _>>()
        .map_err(anyhow::Error::msg)?;

    let mut to_export = Vec::new();
    let mut skipped = 0usize;

    for path in &aseprite_paths {
        let rel_path = export_cache::relative_path_key(path_to_sprites_dir, path)
            .map_err(anyhow::Error::msg)?;
        let file_hash = export_cache::hash_file(path).map_err(anyhow::Error::msg)?;

        if !force_export && cache.is_unchanged(&rel_path, &file_hash) {
            skipped += 1;
            continue;
        }

        to_export.push(path.clone());
    }

    if aseprite_paths.is_empty() {
        println!(
            "No .aseprite files found under {}",
            path_to_sprites_dir.display()
        );
    } else if to_export.is_empty() {
        println!("Skipped {skipped} unchanged .aseprite file(s)");
    } else {
        println!(
            "Skipped {skipped} unchanged, exporting {} .aseprite file(s) (up to {ASEPRITE_EXPORT_THREADS} parallel Aseprite processes)...",
            to_export.len()
        );
    }

    if to_export.is_empty() {
        cache.retain_files(&current_rel_paths);
        cache
            .save(path_to_sprites_dir)
            .map_err(anyhow::Error::msg)?;

        if should_focus_gamemaker {
            gamemaker_window_manip::focus_gamemaker_window(false)?;
        }
        return Ok(());
    }

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(ASEPRITE_EXPORT_THREADS)
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to create thread pool: {e}"))?;

    let script_path = &script_path;
    let watch_dir = path_to_sprites_dir;

    let export_results: Vec<(PathBuf, Result<Vec<PreparedSpriteImport>, String>)> =
        pool.install(|| {
            to_export
                .par_iter()
                .map(|path| {
                    let result = prepare_aseprite_export(path, script_path, watch_dir);
                    (path.clone(), result)
                })
                .collect()
        });

    let mut prepared_imports = Vec::new();
    let mut export_errors = Vec::new();

    for (path, result) in export_results {
        match result {
            Ok(imports) => prepared_imports.extend(imports),
            Err(e) => {
                eprintln!("Error exporting {}: {e}", path.display());
                export_errors.push((path, e));
            }
        }
    }

    if !export_errors.is_empty() {
        return Err(anyhow::anyhow!(
            "{} .aseprite file(s) failed to export",
            export_errors.len()
        ));
    }

    if prepared_imports.is_empty() {
        cache.retain_files(&current_rel_paths);
        cache
            .save(path_to_sprites_dir)
            .map_err(anyhow::Error::msg)?;

        if should_focus_gamemaker {
            gamemaker_window_manip::focus_gamemaker_window(false)?;
        }
        return Ok(());
    }

    let name_entries: Vec<(PathBuf, String)> = prepared_imports
        .iter()
        .map(|import| (import.aseprite_path.clone(), import.tag_name.clone()))
        .collect();

    let to_export_set: HashSet<PathBuf> = to_export.iter().cloned().collect();
    let mut resolution_entries = name_entries.clone();

    for path in &aseprite_paths {
        if to_export_set.contains(path) {
            continue;
        }

        let rel_path = export_cache::relative_path_key(path_to_sprites_dir, path)
            .map_err(anyhow::Error::msg)?;
        let Some(file_cache) = cache.files.get(&rel_path) else {
            continue;
        };

        for tag_name in file_cache.tags.keys() {
            resolution_entries.push((path.clone(), tag_name.clone()));
        }
    }

    let resolved_names = sprites::gm_import::resolve_sprite_names(watch_dir, &resolution_entries)
        .map_err(anyhow::Error::msg)?;

    let name_lookup: HashMap<(PathBuf, String), String> =
        resolution_entries.into_iter().zip(resolved_names).collect();

    let sprite_names: Vec<String> = name_entries
        .iter()
        .map(|entry| {
            name_lookup.get(entry).cloned().ok_or_else(|| {
                anyhow::anyhow!("Missing resolved sprite name for {}", entry.0.display())
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut cache_updates: HashMap<String, (String, HashMap<String, export_cache::TagCacheEntry>)> =
        HashMap::new();

    let import_requests: Vec<sprites::gm_import::SpriteImportRequest> = prepared_imports
        .into_iter()
        .zip(sprite_names)
        .map(|(import, sprite_name)| {
            let rel_path = export_cache::relative_path_key(watch_dir, &import.aseprite_path)
                .map_err(anyhow::Error::msg)?;
            let file_hash =
                export_cache::hash_file(&import.aseprite_path).map_err(anyhow::Error::msg)?;

            let file_entry = cache_updates
                .entry(rel_path)
                .or_insert_with(|| (file_hash, HashMap::new()));

            file_entry.1.insert(
                import.tag_name.clone(),
                export_cache::TagCacheEntry {
                    sprite_name: sprite_name.clone(),
                    gm_folder: import.gm_folder.clone(),
                    width: import.width,
                    height: import.height,
                    frame_count: import.frames.len() as u32,
                },
            );

            Ok(sprites::gm_import::SpriteImportRequest {
                sprite_name,
                frames: import.frames,
                gm_folder_path: import.gm_folder,
                width: import.width,
                height: import.height,
            })
        })
        .collect::<Result<Vec<_>, anyhow::Error>>()?;

    sprites::gm_import::import_sprites_batch(project_path, &import_requests)
        .map_err(anyhow::Error::msg)?;

    for (rel_path, (file_hash, tags)) in cache_updates {
        cache.record_file(rel_path, file_hash, tags);
    }
    cache.retain_files(&current_rel_paths);
    cache
        .save(path_to_sprites_dir)
        .map_err(anyhow::Error::msg)?;

    if should_focus_gamemaker {
        gamemaker_window_manip::focus_gamemaker_window(false)?;
    }

    Ok(())
}
