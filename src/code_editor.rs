use std::fs;
use std::io;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn process_gml_file_change(file: &Path) {
    if let Err(err) = process_gml_file_change_impl(file) {
        eprintln!(
            "Error: Failed to process command comments in '{}': {}",
            file.display(),
            err
        );
    }
}

fn process_gml_file_change_impl(file: &Path) -> io::Result<()> {
    let content = fs::read_to_string(file)?;
    let mut changed = false;
    let mut output_lines = Vec::new();

    for line in content.lines() {
        if let Some(replacement) = try_expand_line_command(line) {
            output_lines.push(replacement);
            changed = true;
        } else {
            output_lines.push(line.to_string());
        }
    }

    if !changed {
        return Ok(());
    }

    let mut new_content = output_lines.join("\n");
    if content.ends_with('\n') {
        new_content.push('\n');
    }

    replace_via_temp(file, new_content.as_bytes())?;
    println!("Expanded command comments in {}", file.display());
    Ok(())
}

/// Write to a sibling `.tmp` file, then rename over `path` (same directory → atomic on Windows).
fn replace_via_temp(path: &Path, contents: &[u8]) -> io::Result<()> {
    let dir = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));

    let name = path.file_name().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "path has no file name",
        )
    })?;

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let tmp_path = dir.join(format!(
        "{}.gmhelper.{}.{}.tmp",
        name.to_string_lossy(),
        std::process::id(),
        stamp
    ));

    fs::write(&tmp_path, contents)?;
    match fs::rename(&tmp_path, path) {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = fs::remove_file(&tmp_path);
            Err(e)
        }
    }
}

fn try_expand_line_command(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    let indent_len = line.len() - trimmed.len();
    let indent = &line[..indent_len];

    // Command format: //: <command>;
    if !trimmed.starts_with("//:") {
        return None;
    }

    let command_with_end = trimmed.trim_start_matches("//:").trim_start();
    let command_end = command_with_end.find(';')?;
    let command = command_with_end[..command_end].trim();
    if command.is_empty() {
        return None;
    }

    expand_command(command).map(|expanded| format!("{indent}{expanded}"))
}

fn expand_command(command: &str) -> Option<String> {
    let mut parts = command.split_whitespace();
    let name = parts.next()?;

    match name {
        "for" => {
            let variable = parts.next()?;
            if parts.next().is_some() {
                return None;
            }
            Some(format!(
                "for(var i = 0;i < array_length({variable}); i++) \n{{ \n}}"
            ))
        }
        _ => None,
    }
}
