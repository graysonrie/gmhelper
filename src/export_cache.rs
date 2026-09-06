use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

pub const CACHE_VERSION: u32 = 1;
const FORCE_EXPORT_ENV: &str = "GMHELPER_FORCE_EXPORT";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct TagCacheEntry {
    pub sprite_name: String,
    pub gm_folder: String,
    pub width: u32,
    pub height: u32,
    pub frame_count: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct FileCacheEntry {
    pub file_hash: String,
    pub tags: HashMap<String, TagCacheEntry>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct ExportCache {
    pub version: u32,
    pub watch_dir: String,
    pub project_path: String,
    pub files: HashMap<String, FileCacheEntry>,
}

impl ExportCache {
    pub fn new(watch_dir: &Path, project_path: &Path) -> Result<Self, String> {
        Ok(Self {
            version: CACHE_VERSION,
            watch_dir: canonical_path_string(watch_dir)?,
            project_path: canonical_path_string(project_path)?,
            files: HashMap::new(),
        })
    }

    pub fn load(watch_dir: &Path, project_path: &Path) -> Result<Self, String> {
        let path = cache_file_path(watch_dir);
        if !path.exists() {
            return Self::new(watch_dir, project_path);
        }

        let content = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read export cache {}: {e}", path.display()))?;
        let cache: Self = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse export cache {}: {e}", path.display()))?;

        let watch_key = canonical_path_string(watch_dir)?;
        let project_key = canonical_path_string(project_path)?;

        if cache.version != CACHE_VERSION
            || cache.watch_dir != watch_key
            || cache.project_path != project_key
        {
            return Self::new(watch_dir, project_path);
        }

        Ok(cache)
    }

    pub fn save(&self, watch_dir: &Path) -> Result<(), String> {
        let path = cache_file_path(watch_dir);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                format!("Failed to create cache directory {}: {e}", parent.display())
            })?;
        }

        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize export cache: {e}"))?;
        fs::write(&path, json)
            .map_err(|e| format!("Failed to write export cache {}: {e}", path.display()))?;

        Ok(())
    }

    pub fn is_unchanged(&self, rel_path: &str, file_hash: &str) -> bool {
        self.files
            .get(rel_path)
            .is_some_and(|entry| entry.file_hash == file_hash)
    }

    pub fn record_file(
        &mut self,
        rel_path: String,
        file_hash: String,
        tags: HashMap<String, TagCacheEntry>,
    ) {
        self.files
            .insert(rel_path, FileCacheEntry { file_hash, tags });
    }

    pub fn retain_files(&mut self, current_rel_paths: &HashSet<String>) {
        self.files
            .retain(|rel_path, _| current_rel_paths.contains(rel_path));
    }
}

#[allow(unused)]
pub fn is_force_export() -> bool {
    std::env::var(FORCE_EXPORT_ENV)
        .map(|value| !value.is_empty())
        .unwrap_or(false)
}

pub fn hash_file(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path)
        .map_err(|e| format!("Failed to read {} for hashing: {e}", path.display()))?;
    Ok(blake3::hash(&bytes).to_hex().to_string())
}

pub fn relative_path_key(watch_dir: &Path, aseprite_path: &Path) -> Result<String, String> {
    let relative = aseprite_path.strip_prefix(watch_dir).map_err(|_| {
        format!(
            "Path {} is not under watch directory {}",
            aseprite_path.display(),
            watch_dir.display()
        )
    })?;

    Ok(relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/"))
}

fn cache_file_path(watch_dir: &Path) -> PathBuf {
    watch_dir.join(".gmhelper").join("export-cache.json")
}

fn canonical_path_string(path: &Path) -> Result<String, String> {
    let canonical = path
        .canonicalize()
        .map_err(|e| format!("Failed to canonicalize {}: {e}", path.display()))?;
    Ok(canonical.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("gmhelper-export-cache-{name}-{stamp}"))
    }

    #[test]
    fn cache_round_trip_serialize() {
        let watch_dir = temp_dir("round-trip-watch");
        let project_path = watch_dir.join("project.yyp");
        fs::create_dir_all(&watch_dir).unwrap();
        fs::write(&project_path, "{}").unwrap();

        let mut cache = ExportCache::new(&watch_dir, &project_path).unwrap();
        let mut tags = HashMap::new();
        tags.insert(
            "idle".to_string(),
            TagCacheEntry {
                sprite_name: "sPlayerIdle".to_string(),
                gm_folder: "Sprites/Characters".to_string(),
                width: 32,
                height: 32,
                frame_count: 4,
            },
        );
        cache.record_file(
            "characters/player.aseprite".to_string(),
            "abc123".to_string(),
            tags,
        );
        cache.save(&watch_dir).unwrap();

        let loaded = ExportCache::load(&watch_dir, &project_path).unwrap();
        assert_eq!(loaded, cache);
        assert!(loaded.is_unchanged("characters/player.aseprite", "abc123"));
        assert!(!loaded.is_unchanged("characters/player.aseprite", "different"));
    }

    #[test]
    fn stale_entry_removal() {
        let watch_dir = temp_dir("stale-watch");
        let project_path = watch_dir.join("project.yyp");
        fs::create_dir_all(&watch_dir).unwrap();
        fs::write(&project_path, "{}").unwrap();

        let mut cache = ExportCache::new(&watch_dir, &project_path).unwrap();
        cache.record_file(
            "old.aseprite".to_string(),
            "hash-old".to_string(),
            HashMap::new(),
        );
        cache.record_file(
            "kept.aseprite".to_string(),
            "hash-kept".to_string(),
            HashMap::new(),
        );

        let current = HashSet::from(["kept.aseprite".to_string()]);
        cache.retain_files(&current);

        assert!(!cache.files.contains_key("old.aseprite"));
        assert!(cache.files.contains_key("kept.aseprite"));
    }

    #[test]
    fn load_invalidates_on_project_mismatch() {
        let watch_dir = temp_dir("project-mismatch-watch");
        let project_a = watch_dir.join("a.yyp");
        let project_b = watch_dir.join("b.yyp");
        fs::create_dir_all(&watch_dir).unwrap();
        fs::write(&project_a, "{}").unwrap();
        fs::write(&project_b, "{}").unwrap();

        let mut cache = ExportCache::new(&watch_dir, &project_a).unwrap();
        cache.record_file(
            "sprite.aseprite".to_string(),
            "hash".to_string(),
            HashMap::new(),
        );
        cache.save(&watch_dir).unwrap();

        let loaded = ExportCache::load(&watch_dir, &project_b).unwrap();
        assert!(loaded.files.is_empty());
    }

    #[test]
    fn force_export_env_is_detected() {
        let key = FORCE_EXPORT_ENV;
        let previous = std::env::var(key).ok();

        // SAFETY: test-only mutation of this process's environment.
        unsafe {
            std::env::set_var(key, "1");
        }
        assert!(is_force_export());

        unsafe {
            std::env::remove_var(key);
        }
        assert!(!is_force_export());

        unsafe {
            match previous {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }

    #[test]
    fn relative_path_key_uses_forward_slashes() {
        let watch_dir = Path::new("/sprites");
        let file = Path::new("/sprites/characters/player.aseprite");
        let key = relative_path_key(watch_dir, file).unwrap();
        assert_eq!(key, "characters/player.aseprite");
    }
}
