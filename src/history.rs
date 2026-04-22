use std::io;
use std::path::PathBuf;
use std::process::ExitStatus;

use serde::{Deserialize, Serialize};

const MAX_ENTRIES: usize = 10;
const FILE_NAME: &str = "command_history.json";
const APP_FOLDER: &str = "gmhelper";

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct CommandHistory {
    /// Newest first: `entries[0]` is the most recent command (#1 in the list).
    pub entries: Vec<HistoryEntry>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct HistoryEntry {
    /// Arguments after the program name, e.g. `["sprites", "--start"]`.
    pub args: Vec<String>,
}

pub fn data_dir() -> io::Result<PathBuf> {
    let base = dirs::data_local_dir().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "local data directory is not available (dirs::data_local_dir returned None)",
        )
    })?;
    Ok(base.join(APP_FOLDER))
}

fn history_path() -> io::Result<PathBuf> {
    Ok(data_dir()?.join(FILE_NAME))
}

pub fn load() -> CommandHistory {
    let Ok(path) = history_path() else {
        return CommandHistory::default();
    };
    let Ok(data) = std::fs::read_to_string(&path) else {
        return CommandHistory::default();
    };
    serde_json::from_str(&data).unwrap_or_default()
}

/// Appends the current process invocation to history (newest first), capped at
/// [MAX_ENTRIES]. Skips if the first argument is `previous`.
pub fn record_current_invocation() -> io::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        return Ok(());
    }
    if args.first().is_some_and(|a| a == "previous") {
        return Ok(());
    }

    let mut history = load();
    history.entries.insert(0, HistoryEntry { args });
    history.entries.truncate(MAX_ENTRIES);

    let dir = data_dir()?;
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(FILE_NAME);
    let json = serde_json::to_string_pretty(&history)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    std::fs::write(&path, json)
}

pub fn list_text(history: &CommandHistory) -> String {
    if history.entries.is_empty() {
        return "No commands in history yet.\n".to_string();
    }
    let mut out = String::new();
    for (i, entry) in history.entries.iter().enumerate() {
        let n = i + 1;
        out.push_str(&format!("#{}  gmhelper {}\n", n, shell_escape_args(&entry.args)));
    }
    out
}

/// Quote args that need it so a human can read them as one line.
fn shell_escape_args(args: &[String]) -> String {
    let mut parts = Vec::with_capacity(args.len());
    for a in args {
        if a.is_empty() {
            parts.push("\"\"".to_string());
        } else if a
            .chars()
            .any(|c| c.is_whitespace() || c == '"' || c == '\\')
        {
            let mut s = String::with_capacity(a.len() + 2);
            s.push('"');
            for c in a.chars() {
                if c == '"' || c == '\\' {
                    s.push('\\');
                }
                s.push(c);
            }
            s.push('"');
            parts.push(s);
        } else {
            parts.push(a.clone());
        }
    }
    parts.join(" ")
}

/// Re-runs the Nth most recent command (`n` in 1..=10). Returns the child's exit status.
pub fn reexecute(n: u8) -> io::Result<ExitStatus> {
    if !(1u8..=10).contains(&n) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("index must be from 1 to 10, got {n}"),
        ));
    }
    let history = load();
    let idx = usize::from(n - 1);
    let Some(entry) = history.entries.get(idx) else {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "no history entry at #{} (only {} in history)",
                n,
                history.entries.len()
            ),
        ));
    };
    let exe = std::env::current_exe()?;
    std::process::Command::new(&exe).args(&entry.args).status()
}
