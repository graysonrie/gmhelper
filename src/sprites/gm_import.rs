use image::DynamicImage;
use std::fs;
use std::path::Path;

use super::bbox::calculate_tight_bbox;
use super::models::gm_project_model::GMFolder;
use super::models::gm_sprite_model::{GMSpriteModel, ResourceReference};

/// Import a set of frames into a GameMaker project as a sprite resource.
///
/// * `project_path`   - path to the `.yyp` file
/// * `sprite_name`    - resource name (e.g. "sPlayerIdle")
/// * `frames`         - the individual frame images (RGBA)
/// * `gm_folder_path` - GameMaker folder path like "Sprites" or "Sprites/Enemies"
/// * `width`/`height` - dimensions of each frame in pixels
pub fn import_sprite_to_project(
    project_path: &Path,
    sprite_name: &str,
    frames: &[DynamicImage],
    gm_folder_path: &str,
    width: u32,
    height: u32,
) -> Result<(), String> {
    let project_dir = project_path
        .parent()
        .ok_or_else(|| "Could not determine project directory from .yyp path".to_string())?;

    // --- 1. Parse the .yyp as a generic Value to preserve exact field order ---
    let yyp_content = fs::read_to_string(project_path)
        .map_err(|e| format!("Failed to read .yyp file: {e}"))?;
    let yyp_clean = strip_trailing_commas(&yyp_content);
    let mut project: serde_json::Value = serde_json::from_str(&yyp_clean)
        .map_err(|e| format!("Failed to parse .yyp JSON: {e}"))?;

    // --- 2. Read overrides from existing sprite if dimensions match ---
    let sprite_dir = project_dir.join("sprites").join(sprite_name);
    let overrides = read_sprite_overrides(&sprite_dir, sprite_name, width, height);

    // Delete existing sprite folder (overwrite strategy)
    if sprite_dir.exists() {
        fs::remove_dir_all(&sprite_dir)
            .map_err(|e| format!("Failed to remove old sprite directory: {e}"))?;
        if overrides.is_some() {
            println!("  Overwriting sprite (preserving bbox/origin): {}", sprite_dir.display());
        } else {
            println!("  Removed existing sprite: {}", sprite_dir.display());
        }
    }

    // --- 3. Generate UUIDs ---
    let layer_guid = uuid::Uuid::new_v4().to_string();
    let frame_guids: Vec<String> = (0..frames.len())
        .map(|_| uuid::Uuid::new_v4().to_string())
        .collect();

    // --- 4. Create directory structure ---
    //   sprites/{sprite_name}/
    //   sprites/{sprite_name}/layers/{frameGuid}/  (one per frame)
    fs::create_dir_all(&sprite_dir)
        .map_err(|e| format!("Failed to create sprite directory: {e}"))?;

    let layers_dir = sprite_dir.join("layers");
    fs::create_dir_all(&layers_dir)
        .map_err(|e| format!("Failed to create layers directory: {e}"))?;

    // --- 5. Save frame PNGs ---
    for (i, frame) in frames.iter().enumerate() {
        let guid = &frame_guids[i];
        let rgba = frame.to_rgba8();

        // Top-level frame image: sprites/{sprite_name}/{guid}.png
        let frame_path = sprite_dir.join(format!("{guid}.png"));
        rgba.save(&frame_path)
            .map_err(|e| format!("Failed to save frame {i} PNG: {e}"))?;

        // Layer copy: sprites/{sprite_name}/layers/{guid}/{layer_guid}.png
        let layer_frame_dir = layers_dir.join(guid);
        fs::create_dir_all(&layer_frame_dir)
            .map_err(|e| format!("Failed to create layer frame directory: {e}"))?;

        let layer_frame_path = layer_frame_dir.join(format!("{layer_guid}.png"));
        rgba.save(&layer_frame_path)
            .map_err(|e| format!("Failed to save layer frame {i} PNG: {e}"))?;
    }

    // --- 6. Calculate bounding box ---
    let bbox = calculate_tight_bbox(frames, width, height);

    // --- 7. Build the parent folder reference ---
    // gm_folder_path is e.g. "Sprites/Enemies"
    // The parent's folderPath in the .yy becomes "folders/Sprites/Enemies.yy"
    let folder_yy_path = format!("folders/{gm_folder_path}.yy");
    let parent_name = gm_folder_path
        .rsplit('/')
        .next()
        .unwrap_or(gm_folder_path);

    let parent_ref = ResourceReference {
        name: parent_name.to_string(),
        path: folder_yy_path,
    };

    // --- 8. Build and write the .yy sprite model ---
    let mut sprite_model = GMSpriteModel::new(
        sprite_name,
        width as i32,
        height as i32,
        &frame_guids,
        &layer_guid,
        parent_ref,
        bbox,
    );

    // If the old sprite had the same dimensions, preserve its bbox/origin settings
    if let Some(ov) = overrides {
        sprite_model.bbox_mode = ov.bbox_mode;
        sprite_model.bbox_bottom = ov.bbox_bottom;
        sprite_model.bbox_left = ov.bbox_left;
        sprite_model.bbox_right = ov.bbox_right;
        sprite_model.bbox_top = ov.bbox_top;
        sprite_model.origin = ov.origin;
        sprite_model.sequence.xorigin = ov.xorigin;
        sprite_model.sequence.yorigin = ov.yorigin;
    }

    let yy_path = sprite_dir.join(format!("{sprite_name}.yy"));
    let yy_json = serde_json::to_string_pretty(&sprite_model)
        .map_err(|e| format!("Failed to serialize sprite .yy: {e}"))?;
    fs::write(&yy_path, &yy_json)
        .map_err(|e| format!("Failed to write sprite .yy: {e}"))?;

    // --- 9. Ensure all folders exist in the .yyp ---
    ensure_gm_folders_value(&mut project, gm_folder_path)?;

    // --- 10. Add/replace the sprite resource in .yyp ---
    let resource_path = format!("sprites/{sprite_name}/{sprite_name}.yy");
    {
        let resources = project
            .get_mut("resources")
            .and_then(|v| v.as_array_mut())
            .ok_or_else(|| "Missing 'resources' array in .yyp".to_string())?;

        // Remove any existing entry with the same name
        resources.retain(|entry| {
            entry
                .get("id")
                .and_then(|id| id.get("name"))
                .and_then(|n| n.as_str())
                != Some(sprite_name)
        });

        // Push the new resource entry
        resources.push(serde_json::json!({
            "id": { "name": sprite_name, "path": resource_path }
        }));
    }

    // --- 11. Write the .yyp back to disk ---
    let yyp_json = serde_json::to_string_pretty(&project)
        .map_err(|e| format!("Failed to serialize .yyp: {e}"))?;
    fs::write(project_path, &yyp_json)
        .map_err(|e| format!("Failed to write .yyp: {e}"))?;

    println!(
        "  Imported sprite '{sprite_name}' ({} frame{}) into {}",
        frames.len(),
        if frames.len() == 1 { "" } else { "s" },
        project_path.display(),
    );

    Ok(())
}

/// Ensure that every intermediate folder in `gm_folder_path` exists in the
/// `.yyp` `Folders` array. For example, `"Sprites/Enemies/Bosses"` will ensure
/// entries for `"Sprites"`, `"Sprites/Enemies"`, and `"Sprites/Enemies/Bosses"`.
/// Operates directly on the `serde_json::Value` to preserve field ordering.
fn ensure_gm_folders_value(
    project: &mut serde_json::Value,
    gm_folder_path: &str,
) -> Result<(), String> {
    let folders = project
        .get_mut("Folders")
        .and_then(|v| v.as_array_mut())
        .ok_or_else(|| "Missing 'Folders' array in .yyp".to_string())?;

    let parts: Vec<&str> = gm_folder_path.split('/').collect();
    let mut accumulated = String::new();

    for part in &parts {
        if accumulated.is_empty() {
            accumulated = (*part).to_string();
        } else {
            accumulated = format!("{accumulated}/{part}");
        }

        let folder_yy_path = format!("folders/{accumulated}.yy");

        let already_exists = folders.iter().any(|f| {
            f.get("folderPath")
                .and_then(|p| p.as_str())
                == Some(&folder_yy_path)
        });

        if !already_exists {
            let folder = GMFolder::new(part, &folder_yy_path);
            let folder_value = serde_json::to_value(&folder)
                .map_err(|e| format!("Failed to serialize folder entry: {e}"))?;
            folders.push(folder_value);
        }
    }

    Ok(())
}

/// Compute the GameMaker folder path by mirroring the filesystem hierarchy
/// between the watched directory and the Aseprite file, nested under "Sprites".
///
/// Returns the GM folder path (e.g. "Sprites/Characters/Enemies") and uses
/// CamelCase for each directory component.
pub fn compute_gm_folder_path(watch_dir: &Path, aseprite_path: &Path) -> String {
    let relative = aseprite_path
        .parent()
        .and_then(|p| p.strip_prefix(watch_dir).ok());

    match relative {
        Some(rel) if rel.components().next().is_some() => {
            let parts: Vec<String> = rel
                .components()
                .map(|c| to_camel_case(&c.as_os_str().to_string_lossy()))
                .collect();
            format!("Sprites/{}", parts.join("/"))
        }
        _ => "Sprites".to_string(),
    }
}

/// Convert a string to CamelCase, splitting on `_`, `-`, `.`, and spaces.
/// Each word's first letter is capitalized, the rest lowered.
fn to_camel_case(s: &str) -> String {
    s.split(|c: char| c == '_' || c == '-' || c == '.' || c == ' ')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => {
                    let upper: String = first.to_uppercase().collect();
                    let rest: String = chars.collect::<String>().to_lowercase();
                    format!("{upper}{rest}")
                }
                None => String::new(),
            }
        })
        .collect()
}

/// Derive the GameMaker sprite name from the Aseprite filename and tag name.
/// Follows the existing Lua convention: `s{FileCamelCase}{TagCamelCase}`.
pub fn derive_sprite_name(aseprite_path: &Path, tag_name: &str) -> Result<String, String> {
    let file_stem = aseprite_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| "Could not extract filename from Aseprite path".to_string())?;

    let file_part = to_camel_case(file_stem);
    let tag_part = to_camel_case(tag_name);

    Ok(format!("s{file_part}{tag_part}"))
}

/// Fields preserved from an existing sprite `.yy` when dimensions match.
struct SpriteOverrides {
    bbox_mode: i32,
    bbox_bottom: i32,
    bbox_left: i32,
    bbox_right: i32,
    bbox_top: i32,
    origin: i32,
    xorigin: i32,
    yorigin: i32,
}

/// Try to read bbox and origin overrides from an existing sprite's `.yy` file.
/// Returns `Some(overrides)` only if the file exists and its width/height match
/// the new sprite dimensions, meaning the overrides are still valid.
fn read_sprite_overrides(
    sprite_dir: &Path,
    sprite_name: &str,
    new_width: u32,
    new_height: u32,
) -> Option<SpriteOverrides> {
    let yy_path = sprite_dir.join(format!("{sprite_name}.yy"));
    let content = fs::read_to_string(&yy_path).ok()?;
    let clean = strip_trailing_commas(&content);
    let val: serde_json::Value = serde_json::from_str(&clean).ok()?;

    let old_width = val.get("width")?.as_i64()?;
    let old_height = val.get("height")?.as_i64()?;

    if old_width != new_width as i64 || old_height != new_height as i64 {
        return None;
    }

    let seq = val.get("sequence")?;

    Some(SpriteOverrides {
        bbox_mode: val.get("bboxMode")?.as_i64()? as i32,
        bbox_bottom: val.get("bbox_bottom")?.as_i64()? as i32,
        bbox_left: val.get("bbox_left")?.as_i64()? as i32,
        bbox_right: val.get("bbox_right")?.as_i64()? as i32,
        bbox_top: val.get("bbox_top")?.as_i64()? as i32,
        origin: val.get("origin")?.as_i64()? as i32,
        xorigin: seq.get("xorigin")?.as_i64()? as i32,
        yorigin: seq.get("yorigin")?.as_i64()? as i32,
    })
}

/// Remove trailing commas from JSON text (commas before `]` or `}`).
/// GameMaker's JSON files commonly include trailing commas which standard
/// JSON parsers reject.
fn strip_trailing_commas(json: &str) -> String {
    let mut result = String::with_capacity(json.len());
    let mut in_string = false;
    let mut escape_next = false;
    let chars: Vec<char> = json.chars().collect();

    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];

        if escape_next {
            result.push(c);
            escape_next = false;
            i += 1;
            continue;
        }

        if c == '\\' && in_string {
            result.push(c);
            escape_next = true;
            i += 1;
            continue;
        }

        if c == '"' {
            in_string = !in_string;
            result.push(c);
            i += 1;
            continue;
        }

        if !in_string && c == ',' {
            // Look ahead past whitespace for ] or }
            let mut j = i + 1;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            if j < chars.len() && (chars[j] == ']' || chars[j] == '}') {
                // Skip the trailing comma
                i += 1;
                continue;
            }
        }

        result.push(c);
        i += 1;
    }

    result
}
