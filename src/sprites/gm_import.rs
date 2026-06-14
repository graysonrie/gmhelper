use image::DynamicImage;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use super::bbox::calculate_tight_bbox;
use super::models::gm_project_model::GMFolder;
use super::models::gm_sprite_model::{GMSpriteModel, ResourceReference};

pub struct SpriteImportRequest {
    pub sprite_name: String,
    pub frames: Vec<DynamicImage>,
    pub gm_folder_path: String,
    pub width: u32,
    pub height: u32,
}

/// Import a set of frames into a GameMaker project as a sprite resource.
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

    let yyp_content =
        fs::read_to_string(project_path).map_err(|e| format!("Failed to read .yyp file: {e}"))?;
    let yyp_clean = strip_trailing_commas(&yyp_content);
    let mut project: serde_json::Value =
        serde_json::from_str(&yyp_clean).map_err(|e| format!("Failed to parse .yyp JSON: {e}"))?;

    write_sprite_files(
        project_dir,
        sprite_name,
        frames,
        gm_folder_path,
        width,
        height,
    )?;
    register_sprite_in_project(&mut project, sprite_name, gm_folder_path)?;

    let yyp_json = serde_json::to_string_pretty(&project)
        .map_err(|e| format!("Failed to serialize .yyp: {e}"))?;
    fs::write(project_path, &yyp_json).map_err(|e| format!("Failed to write .yyp: {e}"))?;

    println!(
        "  Imported sprite '{sprite_name}' ({} frame{}) into {}",
        frames.len(),
        if frames.len() == 1 { "" } else { "s" },
        project_path.display(),
    );

    Ok(())
}

/// Import many sprites with a single `.yyp` read and write.
pub fn import_sprites_batch(
    project_path: &Path,
    imports: &[SpriteImportRequest],
) -> Result<(), String> {
    if imports.is_empty() {
        return Ok(());
    }

    let project_dir = project_path
        .parent()
        .ok_or_else(|| "Could not determine project directory from .yyp path".to_string())?;

    let yyp_content =
        fs::read_to_string(project_path).map_err(|e| format!("Failed to read .yyp file: {e}"))?;
    let yyp_clean = strip_trailing_commas(&yyp_content);
    let mut project: serde_json::Value =
        serde_json::from_str(&yyp_clean).map_err(|e| format!("Failed to parse .yyp JSON: {e}"))?;

    for import in imports {
        write_sprite_files(
            project_dir,
            &import.sprite_name,
            &import.frames,
            &import.gm_folder_path,
            import.width,
            import.height,
        )?;
        register_sprite_in_project(&mut project, &import.sprite_name, &import.gm_folder_path)?;

        println!(
            "  Imported sprite '{}' ({} frame{})",
            import.sprite_name,
            import.frames.len(),
            if import.frames.len() == 1 { "" } else { "s" },
        );
    }

    let yyp_json = serde_json::to_string_pretty(&project)
        .map_err(|e| format!("Failed to serialize .yyp: {e}"))?;
    fs::write(project_path, &yyp_json).map_err(|e| format!("Failed to write .yyp: {e}"))?;

    println!(
        "  Updated {} sprite resource(s) in {}",
        imports.len(),
        project_path.display(),
    );

    Ok(())
}

fn write_sprite_files(
    project_dir: &Path,
    sprite_name: &str,
    frames: &[DynamicImage],
    gm_folder_path: &str,
    width: u32,
    height: u32,
) -> Result<(), String> {
    let sprite_dir = project_dir.join("sprites").join(sprite_name);
    let overrides = read_sprite_overrides(&sprite_dir, sprite_name, width, height);

    if sprite_dir.exists() {
        fs::remove_dir_all(&sprite_dir)
            .map_err(|e| format!("Failed to remove old sprite directory: {e}"))?;
        if overrides.is_some() {
            println!(
                "  Overwriting sprite (preserving bbox/origin): {}",
                sprite_dir.display()
            );
        } else {
            println!("  Removed existing sprite: {}", sprite_dir.display());
        }
    }

    let layer_guid = uuid::Uuid::new_v4().to_string();
    let frame_guids: Vec<String> = (0..frames.len())
        .map(|_| uuid::Uuid::new_v4().to_string())
        .collect();

    fs::create_dir_all(&sprite_dir)
        .map_err(|e| format!("Failed to create sprite directory: {e}"))?;

    let layers_dir = sprite_dir.join("layers");
    fs::create_dir_all(&layers_dir)
        .map_err(|e| format!("Failed to create layers directory: {e}"))?;

    for (i, frame) in frames.iter().enumerate() {
        let guid = &frame_guids[i];
        let rgba = frame.to_rgba8();

        let frame_path = sprite_dir.join(format!("{guid}.png"));
        rgba.save(&frame_path)
            .map_err(|e| format!("Failed to save frame {i} PNG: {e}"))?;

        let layer_frame_dir = layers_dir.join(guid);
        fs::create_dir_all(&layer_frame_dir)
            .map_err(|e| format!("Failed to create layer frame directory: {e}"))?;

        let layer_frame_path = layer_frame_dir.join(format!("{layer_guid}.png"));
        rgba.save(&layer_frame_path)
            .map_err(|e| format!("Failed to save layer frame {i} PNG: {e}"))?;
    }

    let bbox = calculate_tight_bbox(frames, width, height);

    let folder_yy_path = format!("folders/{gm_folder_path}.yy");
    let parent_name = gm_folder_path.rsplit('/').next().unwrap_or(gm_folder_path);

    let parent_ref = ResourceReference {
        name: parent_name.to_string(),
        path: folder_yy_path,
    };

    let mut sprite_model = GMSpriteModel::new(
        sprite_name,
        width as i32,
        height as i32,
        &frame_guids,
        &layer_guid,
        parent_ref,
        bbox,
    );

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
    fs::write(&yy_path, &yy_json).map_err(|e| format!("Failed to write sprite .yy: {e}"))?;

    Ok(())
}

fn register_sprite_in_project(
    project: &mut serde_json::Value,
    sprite_name: &str,
    gm_folder_path: &str,
) -> Result<(), String> {
    ensure_gm_folders_value(project, gm_folder_path)?;

    let resource_path = format!("sprites/{sprite_name}/{sprite_name}.yy");
    let resources = project
        .get_mut("resources")
        .and_then(|v| v.as_array_mut())
        .ok_or_else(|| "Missing 'resources' array in .yyp".to_string())?;

    resources.retain(|entry| {
        entry
            .get("id")
            .and_then(|id| id.get("name"))
            .and_then(|n| n.as_str())
            != Some(sprite_name)
    });

    resources.push(serde_json::json!({
        "id": { "name": sprite_name, "path": resource_path }
    }));

    Ok(())
}

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

        let already_exists = folders
            .iter()
            .any(|f| f.get("folderPath").and_then(|p| p.as_str()) == Some(&folder_yy_path));

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

fn to_camel_case(s: &str) -> String {
    s.split(['_', '-', '.', ' '])
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

/// Base sprite name: `s{FileCamelCase}{TagCamelCase}`.
pub fn base_sprite_name(file_stem: &str, tag_name: &str) -> String {
    let file_part = to_camel_case(file_stem);
    let tag_part = to_camel_case(tag_name);
    format!("s{file_part}{tag_part}")
}

/// Disambiguated sprite name: `s{Prefix}{FileCamelCase}{TagCamelCase}`.
pub fn prefixed_sprite_name(prefix: &str, file_stem: &str, tag_name: &str) -> String {
    let file_part = to_camel_case(file_stem);
    let tag_part = to_camel_case(tag_name);
    format!("s{prefix}{file_part}{tag_part}")
}

/// Derive the GameMaker sprite name from the Aseprite filename and tag name.
pub fn derive_sprite_name(aseprite_path: &Path, tag_name: &str) -> Result<String, String> {
    let file_stem = aseprite_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| "Could not extract filename from Aseprite path".to_string())?;

    Ok(base_sprite_name(file_stem, tag_name))
}

fn relative_folder_components(watch_dir: &Path, aseprite_path: &Path) -> Vec<String> {
    let Some(rel) = aseprite_path
        .parent()
        .and_then(|p| p.strip_prefix(watch_dir).ok())
    else {
        return Vec::new();
    };

    rel.components()
        .map(|c| to_camel_case(&c.as_os_str().to_string_lossy()))
        .collect()
}

fn folder_prefix_for_depth(watch_dir: &Path, aseprite_path: &Path, depth: usize) -> Option<String> {
    if depth == 0 {
        return None;
    }

    let parts = relative_folder_components(watch_dir, aseprite_path);
    if parts.is_empty() || depth > parts.len() {
        return None;
    }

    let start = parts.len() - depth;
    Some(parts[start..].concat())
}

fn duplicate_indices(names: &[String]) -> HashSet<usize> {
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for name in names {
        *counts.entry(name.as_str()).or_insert(0) += 1;
    }

    names
        .iter()
        .enumerate()
        .filter(|(_, name)| counts.get(name.as_str()).copied().unwrap_or(0) > 1)
        .map(|(idx, _)| idx)
        .collect()
}

/// Resolve unique GameMaker sprite names for a batch of exports.
///
/// When base names collide, prefixes the watch-relative folder path (starting with
/// the immediate parent folder, then escalating with more ancestors).
pub fn resolve_sprite_names(
    watch_dir: &Path,
    entries: &[(PathBuf, String)],
) -> Result<Vec<String>, String> {
    let file_stems: Vec<String> = entries
        .iter()
        .map(|(path, _)| {
            path.file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string())
                .ok_or_else(|| format!("Could not extract filename from {}", path.display()))
        })
        .collect::<Result<_, _>>()?;

    let base_names: Vec<String> = entries
        .iter()
        .enumerate()
        .map(|(i, (_, tag))| base_sprite_name(&file_stems[i], tag))
        .collect();

    let mut base_counts: HashMap<&str, usize> = HashMap::new();
    for name in &base_names {
        *base_counts.entry(name.as_str()).or_insert(0) += 1;
    }

    let mut needs_disambiguation: HashSet<usize> = base_names
        .iter()
        .enumerate()
        .filter(|(_, name)| base_counts.get(name.as_str()).copied().unwrap_or(0) > 1)
        .map(|(idx, _)| idx)
        .collect();

    let mut prefix_depth: Vec<usize> = (0..entries.len())
        .map(|idx| {
            if needs_disambiguation.contains(&idx) {
                1
            } else {
                0
            }
        })
        .collect();

    let mut names = base_names;

    loop {
        for idx in needs_disambiguation.iter().copied() {
            let depth = prefix_depth[idx];
            let prefix = folder_prefix_for_depth(watch_dir, &entries[idx].0, depth).ok_or_else(
                || {
                    format!(
                        "Cannot disambiguate sprite name for {} (tag '{}'): no parent folder relative to {}",
                        entries[idx].0.display(),
                        entries[idx].1,
                        watch_dir.display(),
                    )
                },
            )?;
            names[idx] = prefixed_sprite_name(&prefix, &file_stems[idx], &entries[idx].1);
        }

        let dupes = duplicate_indices(&names);
        if dupes.is_empty() {
            return Ok(names);
        }

        let mut progressed = false;
        for idx in dupes.iter().copied() {
            let max_depth = relative_folder_components(watch_dir, &entries[idx].0).len();
            if prefix_depth[idx] < max_depth {
                prefix_depth[idx] += 1;
                needs_disambiguation.insert(idx);
                progressed = true;
            }
        }

        if !progressed {
            let mut conflict_lines: Vec<String> = dupes
                .into_iter()
                .map(|idx| {
                    format!(
                        "  {} (tag '{}') -> {}",
                        entries[idx].0.display(),
                        entries[idx].1,
                        names[idx],
                    )
                })
                .collect();
            conflict_lines.sort();
            conflict_lines.dedup();
            return Err(format!(
                "Unresolved sprite name conflicts after exhausting folder prefixes:\n{}",
                conflict_lines.join("\n"),
            ));
        }
    }
}

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
            let mut j = i + 1;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            if j < chars.len() && (chars[j] == ']' || chars[j] == '}') {
                i += 1;
                continue;
            }
        }

        result.push(c);
        i += 1;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(watch: &Path, rel: &str, tag: &str) -> (PathBuf, String) {
        (watch.join(rel), tag.to_string())
    }

    #[test]
    fn resolve_no_conflict_uses_base_names() {
        let watch = Path::new("/sprites");
        let entries = vec![
            entry(watch, "characters/player.aseprite", "idle"),
            entry(watch, "enemies/slime.aseprite", "walk"),
        ];

        let names = resolve_sprite_names(watch, &entries).unwrap();
        assert_eq!(names, vec!["sPlayerIdle", "sSlimeWalk"]);
    }

    #[test]
    fn resolve_parent_folder_disambiguation() {
        let watch = Path::new("/sprites");
        let entries = vec![
            entry(watch, "characters/player.aseprite", "idle"),
            entry(watch, "enemies/player.aseprite", "idle"),
        ];

        let names = resolve_sprite_names(watch, &entries).unwrap();
        assert_eq!(names, vec!["sCharactersPlayerIdle", "sEnemiesPlayerIdle"]);
    }

    #[test]
    fn resolve_unique_names_unaffected_by_unrelated_collision() {
        let watch = Path::new("/sprites");
        let entries = vec![
            entry(watch, "characters/player.aseprite", "idle"),
            entry(watch, "enemies/player.aseprite", "idle"),
            entry(watch, "ui/button.aseprite", "press"),
        ];

        let names = resolve_sprite_names(watch, &entries).unwrap();
        assert_eq!(
            names,
            vec![
                "sCharactersPlayerIdle",
                "sEnemiesPlayerIdle",
                "sButtonPress"
            ],
        );
    }

    #[test]
    fn resolve_escalates_when_immediate_parent_insufficient() {
        let watch = Path::new("/sprites");
        let entries = vec![
            entry(watch, "a/npc/player.aseprite", "idle"),
            entry(watch, "b/npc/player.aseprite", "idle"),
        ];

        let names = resolve_sprite_names(watch, &entries).unwrap();
        assert_eq!(names, vec!["sANpcPlayerIdle", "sBNpcPlayerIdle"]);
    }
}
