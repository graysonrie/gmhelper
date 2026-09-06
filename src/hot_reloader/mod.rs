use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::time::{Duration, Instant};
pub mod paths;

use crate::code_editor;
use crate::types::GameMakerVersion;

unsafe extern "system" {
    fn GetForegroundWindow() -> isize;
    fn SetForegroundWindow(hwnd: isize) -> i32;
}

const RUNNER_EXE: &str = "Runner.exe";
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const DEBOUNCE: Duration = Duration::from_millis(500);
const POLL_INTERVAL: Duration = Duration::from_millis(100);

pub fn run_reload(
    yyp_path: PathBuf,
    shutdown: &AtomicBool,
    gamemaker_version: &GameMakerVersion,
) -> anyhow::Result<()> {
    if !yyp_path.exists() {
        anyhow::bail!("Project file '{}' does not exist", yyp_path.display());
    }

    match yyp_path.extension().and_then(|e| e.to_str()) {
        Some("yyp") => {}
        _ => {
            anyhow::bail!(
                "'{}' is not a .yyp file. Provide a valid GameMaker project file.",
                yyp_path.display()
            );
        }
    }

    let project_dir = yyp_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Could not determine project directory from .yyp path"))?
        .to_path_buf();

    println!("Hot-reloading project: {}", yyp_path.display());
    println!("Watching for .gml changes in: {}", project_dir.display());
    println!("Press Ctrl+C to stop...\n");

    let (tx, rx) = mpsc::channel();

    let mut watcher =
        RecommendedWatcher::new(tx, Config::default()).map_err(|e| anyhow::anyhow!("{e}"))?;

    watcher
        .watch(&project_dir, RecursiveMode::Recursive)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let mut pending_reload = false;
    let mut last_change: Option<Instant> = None;

    loop {
        if shutdown.load(Ordering::Relaxed) {
            break;
        }

        match rx.recv_timeout(POLL_INTERVAL) {
            Ok(Ok(event)) => {
                if let EventKind::Modify(_) | EventKind::Create(_) = event.kind {
                    let gml_paths: Vec<&PathBuf> = event
                        .paths
                        .iter()
                        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("gml"))
                        .collect();

                    if !gml_paths.is_empty() {
                        pending_reload = true;
                        last_change = Some(Instant::now());
                        for path in gml_paths {
                            code_editor::process_gml_file_change(path);
                        }
                    }
                }
            }
            Ok(Err(e)) => eprintln!("Watch error: {e}"),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }

        if pending_reload
            && let Some(t) = last_change
            && t.elapsed() >= DEBOUNCE
        {
            pending_reload = false;
            last_change = None;
            println!("Detected .gml change, reloading...");
            kill_runner();
            if let Err(error) = build_and_run(&yyp_path, gamemaker_version) {
                eprintln!("  {error}");
            }
        }
    }

    Ok(())
}

fn kill_runner() {
    let result = Command::new("taskkill")
        .args(["/F", "/IM", RUNNER_EXE])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    match result {
        Ok(output) if output.status.success() => {
            println!("  Killed existing {RUNNER_EXE}");
        }
        _ => {
            // Runner wasn't running or taskkill failed -- either way, proceed
        }
    }
}

fn build_and_run(yyp_path: &Path, gamemaker_version: &GameMakerVersion) -> anyhow::Result<()> {
    let saved_hwnd = unsafe { GetForegroundWindow() };

    let igor_path = paths::get_igor_path(gamemaker_version)?;
    let runtime_root = paths::get_runtime_root(gamemaker_version)?;
    let user_folder = paths::get_user_folder(gamemaker_version)?;
    let cache_dir = paths::get_igor_cache_dir(gamemaker_version)?;
    let temp_dir = paths::get_igor_temp_dir(gamemaker_version)?;

    let result = Command::new(igor_path)
        .arg("/j=8")
        .arg("/v")
        .arg(format!("/project={}", yyp_path.display()))
        .arg(format!("/rp={}", runtime_root.display()))
        .arg(format!("/uf={}", user_folder.display()))
        .arg(format!("/cache={}", cache_dir.display()))
        .arg(format!("/temp={}", temp_dir.display()))
        .arg("--")
        .arg("Windows")
        .arg("Run")
        .creation_flags(CREATE_NO_WINDOW)
        .spawn();

    match result {
        Ok(_) => {
            println!(
                "  Build + run launched for {}",
                yyp_path.file_name().unwrap_or_default().to_string_lossy()
            );

            // Prevent Runner.exe from stealing focus: poll until the foreground
            // window changes (Runner appeared), then immediately restore the
            // original window.
            if saved_hwnd != 0 {
                std::thread::spawn(move || {
                    let timeout = Duration::from_secs(15);
                    let start = Instant::now();
                    while start.elapsed() < timeout {
                        std::thread::sleep(Duration::from_millis(500));
                        let current = unsafe { GetForegroundWindow() };
                        if current != saved_hwnd {
                            unsafe {
                                SetForegroundWindow(saved_hwnd);
                            }
                            break;
                        }
                    }
                });
            }
            Ok(())
        }
        Err(e) => Err(anyhow::anyhow!("Error: Failed to launch Igor.exe: {e}")),
    }
}
