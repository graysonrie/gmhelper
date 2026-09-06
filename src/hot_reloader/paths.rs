use std::{
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

use walkdir::WalkDir;

use crate::types::GameMakerVersion;

pub fn channel_folder(version: &GameMakerVersion) -> &'static str {
    match version {
        GameMakerVersion::GMBETA => "GameMakerStudio2-Beta",
        GameMakerVersion::GMLTS2026 => "GameMakerStudio2-LTS2026",
    }
}

pub fn get_runtime_root(version: &GameMakerVersion) -> anyhow::Result<PathBuf> {
    let program_data = std::env::var("ProgramData")
        .map_err(|_| anyhow::anyhow!("ProgramData environment variable is not set"))?;
    let runtimes_dir = PathBuf::from(program_data)
        .join(channel_folder(version))
        .join("Cache")
        .join("runtimes");

    if !runtimes_dir.is_dir() {
        anyhow::bail!(
            "GameMaker runtimes folder not found at '{}'",
            runtimes_dir.display()
        );
    }

    let mut best: Option<(Vec<u32>, PathBuf)> = None;

    for entry in fs::read_dir(&runtimes_dir).map_err(|e| {
        anyhow::anyhow!(
            "Failed to read runtimes folder '{}': {e}",
            runtimes_dir.display()
        )
    })? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let Some(version_parts) = parse_runtime_version(name) else {
            continue;
        };

        let is_better = best
            .as_ref()
            .is_none_or(|(current, _)| version_parts > *current);
        if is_better {
            best = Some((version_parts, path));
        }
    }

    best.map(|(_, path)| path).ok_or_else(|| {
        anyhow::anyhow!(
            "No GameMaker runtimes found in '{}'",
            runtimes_dir.display()
        )
    })
}

pub fn get_igor_path(version: &GameMakerVersion) -> anyhow::Result<PathBuf> {
    let igor = get_runtime_root(version)?
        .join("bin")
        .join("igor")
        .join("windows")
        .join("x64")
        .join("Igor.exe");

    if !igor.is_file() {
        anyhow::bail!("Igor.exe not found at '{}'", igor.display());
    }

    Ok(igor)
}

pub fn get_user_folder(version: &GameMakerVersion) -> anyhow::Result<PathBuf> {
    let app_data =
        dirs::data_dir().ok_or_else(|| anyhow::anyhow!("Could not determine AppData directory"))?;
    let channel_dir = app_data.join(channel_folder(version));

    if !channel_dir.is_dir() {
        anyhow::bail!(
            "GameMaker user folder not found at '{}'. Log into the GameMaker IDE at least once.",
            channel_dir.display()
        );
    }

    let mut best: Option<(SystemTime, PathBuf)> = None;

    for entry in fs::read_dir(&channel_dir).map_err(|e| {
        anyhow::anyhow!(
            "Failed to read GameMaker user folder '{}': {e}",
            channel_dir.display()
        )
    })? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if !has_licence_file(&path) {
            continue;
        }

        let modified = entry
            .metadata()
            .and_then(|meta| meta.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);

        let is_better = best.as_ref().is_none_or(|(current, _)| modified > *current);
        if is_better {
            best = Some((modified, path));
        }
    }

    best.map(|(_, path)| path).ok_or_else(|| {
        anyhow::anyhow!(
            "No GameMaker licence folder found in '{}'. Log into the GameMaker IDE at least once.",
            channel_dir.display()
        )
    })
}

pub fn get_igor_cache_dir(version: &GameMakerVersion) -> anyhow::Result<PathBuf> {
    ensure_local_subdir(version, "igor-cache")
}

pub fn get_igor_temp_dir(version: &GameMakerVersion) -> anyhow::Result<PathBuf> {
    ensure_local_subdir(version, "igor-temp")
}

pub fn detect_gamemaker_version(yyp_path: &Path) -> anyhow::Result<GameMakerVersion> {
    if !yyp_path.is_file() {
        anyhow::bail!("Project file '{}' does not exist", yyp_path.display());
    }

    let project_stem = yyp_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow::anyhow!("Could not determine project name from .yyp path"))?;

    let local_app_data = dirs::data_local_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine LocalAppData directory"))?;

    let beta_trace = find_project_trace(
        &local_app_data.join(channel_folder(&GameMakerVersion::GMBETA)),
        project_stem,
    );
    let lts_trace = find_project_trace(
        &local_app_data.join(channel_folder(&GameMakerVersion::GMLTS2026)),
        project_stem,
    );

    match (beta_trace, lts_trace) {
        (Some(beta), Some(lts)) => {
            if beta >= lts {
                Ok(GameMakerVersion::GMBETA)
            } else {
                Ok(GameMakerVersion::GMLTS2026)
            }
        }
        (Some(_), None) => Ok(GameMakerVersion::GMBETA),
        (None, Some(_)) => Ok(GameMakerVersion::GMLTS2026),
        (None, None) => detect_from_yyp_ide_version(yyp_path),
    }
}

fn ensure_local_subdir(version: &GameMakerVersion, name: &str) -> anyhow::Result<PathBuf> {
    let local_app_data = dirs::data_local_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine LocalAppData directory"))?;
    let dir = local_app_data.join(channel_folder(version)).join(name);
    fs::create_dir_all(&dir)
        .map_err(|e| anyhow::anyhow!("Failed to create '{}': {e}", dir.display()))?;
    Ok(dir)
}

fn has_licence_file(user_folder: &Path) -> bool {
    user_folder.join("licence.plist").is_file() || user_folder.join("license.plist").is_file()
}

fn parse_runtime_version(name: &str) -> Option<Vec<u32>> {
    let rest = name.strip_prefix("runtime-")?;
    let mut parts = Vec::new();
    for part in rest.split('.') {
        parts.push(part.parse().ok()?);
    }
    if parts.is_empty() { None } else { Some(parts) }
}

fn find_project_trace(channel_local: &Path, project_stem: &str) -> Option<SystemTime> {
    if !channel_local.is_dir() {
        return None;
    }

    let stem_lower = project_stem.to_lowercase();
    let prefix = format!("{stem_lower}_");
    let mut best: Option<SystemTime> = None;

    for entry in WalkDir::new(channel_local)
        .max_depth(6)
        .into_iter()
        .filter_map(|entry| entry.ok())
    {
        let name = entry.file_name().to_string_lossy().to_lowercase();
        if name != stem_lower && !name.starts_with(&prefix) {
            continue;
        }

        let Ok(meta) = entry.metadata() else {
            continue;
        };
        let Ok(modified) = meta.modified() else {
            continue;
        };

        best = Some(best.map_or(modified, |current| current.max(modified)));
    }

    best
}

fn detect_from_yyp_ide_version(yyp_path: &Path) -> anyhow::Result<GameMakerVersion> {
    let text = fs::read_to_string(yyp_path)
        .map_err(|e| anyhow::anyhow!("Failed to read '{}': {e}", yyp_path.display()))?;
    let value: serde_json::Value = parse_gm_json(&text)
        .map_err(|e| anyhow::anyhow!("Failed to parse '{}': {e}", yyp_path.display()))?;
    let ide_version = value
        .get("MetaData")
        .and_then(|meta| meta.get("IDEVersion"))
        .and_then(|version| version.as_str())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Could not determine GameMaker channel for '{}': no MetaData.IDEVersion",
                yyp_path.display()
            )
        })?;

    version_from_ide_version(ide_version).ok_or_else(|| {
        anyhow::anyhow!("Could not determine GameMaker channel from IDEVersion '{ide_version}'")
    })
}

/// Strip trailing commas before `}` / `]` so GameMaker yy-like JSON parses with serde_json.
fn parse_gm_json<T: serde::de::DeserializeOwned>(raw: &str) -> Result<T, anyhow::Error> {
    let mut out = String::with_capacity(raw.len());
    let bytes = raw.as_bytes();
    let mut i = 0;
    let mut in_string = false;
    let mut escape = false;

    while i < bytes.len() {
        let b = bytes[i];
        if in_string {
            out.push(b as char);
            if escape {
                escape = false;
            } else if b == b'\\' {
                escape = true;
            } else if b == b'"' {
                in_string = false;
            }
            i += 1;
            continue;
        }

        if b == b'"' {
            in_string = true;
            out.push('"');
            i += 1;
            continue;
        }

        if b == b',' {
            let mut j = i + 1;
            while j < bytes.len() && matches!(bytes[j], b' ' | b'\t' | b'\n' | b'\r') {
                j += 1;
            }
            if j < bytes.len() && matches!(bytes[j], b'}' | b']') {
                i += 1;
                continue;
            }
        }

        out.push(b as char);
        i += 1;
    }

    serde_json::from_str(&out).map_err(|e| anyhow::anyhow!(e))
}

fn version_from_ide_version(ide_version: &str) -> Option<GameMakerVersion> {
    let parts: Vec<u32> = ide_version
        .split('.')
        .filter_map(|part| part.parse().ok())
        .collect();
    if parts.len() < 2 {
        return None;
    }

    if parts[1] >= 1000 {
        Some(GameMakerVersion::GMBETA)
    } else if parts[0] >= 2026 {
        Some(GameMakerVersion::GMLTS2026)
    } else {
        None
    }
}
