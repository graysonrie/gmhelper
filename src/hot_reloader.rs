use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;
use std::time::{Duration, Instant};

const IGOR_PATH: &str = r"C:\ProgramData\GameMakerStudio2-Beta\Cache\runtimes\runtime-2024.1400.4.968\bin\igor\windows\x64\Igor.exe";
const BUILD_BFF_PATH: &str = r"C:\Users\grays\AppData\Local\GameMakerStudio2-Beta\GMS2TEMP\build.bff";
const RUNNER_EXE: &str = "Runner.exe";
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const DEBOUNCE: Duration = Duration::from_secs(1);
const POLL_INTERVAL: Duration = Duration::from_millis(200);

pub fn run_reload(yyp_path: PathBuf) {
    if !yyp_path.exists() {
        eprintln!("Error: Project file '{}' does not exist", yyp_path.display());
        std::process::exit(1);
    }

    match yyp_path.extension().and_then(|e| e.to_str()) {
        Some("yyp") => {}
        _ => {
            eprintln!(
                "Error: '{}' is not a .yyp file. Provide a valid GameMaker project file.",
                yyp_path.display()
            );
            std::process::exit(1);
        }
    }

    let project_dir = yyp_path
        .parent()
        .unwrap_or_else(|| {
            eprintln!("Error: Could not determine project directory from .yyp path");
            std::process::exit(1);
        })
        .to_path_buf();

    println!("Hot-reloading project: {}", yyp_path.display());
    println!("Watching for .gml changes in: {}", project_dir.display());
    println!("Press Ctrl+C to stop...\n");

    let (tx, rx) = mpsc::channel();

    let mut watcher =
        RecommendedWatcher::new(tx, Config::default()).expect("Failed to create file watcher");

    watcher
        .watch(&project_dir, RecursiveMode::Recursive)
        .expect("Failed to watch project directory");

    let mut pending_reload = false;
    let mut last_change: Option<Instant> = None;

    loop {
        match rx.recv_timeout(POLL_INTERVAL) {
            Ok(Ok(event)) => {
                if let EventKind::Modify(_) | EventKind::Create(_) = event.kind {
                    let has_gml = event
                        .paths
                        .iter()
                        .any(|p| p.extension().and_then(|e| e.to_str()) == Some("gml"));

                    if has_gml {
                        pending_reload = true;
                        last_change = Some(Instant::now());
                    }
                }
            }
            Ok(Err(e)) => eprintln!("Watch error: {e}"),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }

        if pending_reload {
            if let Some(t) = last_change {
                if t.elapsed() >= DEBOUNCE {
                    pending_reload = false;
                    last_change = None;
                    println!("Detected .gml change, reloading...");
                    kill_runner();
                    build_and_run(&yyp_path);
                }
            }
        }
    }
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

fn build_and_run(yyp_path: &Path) {
    let options_arg = format!("-options={BUILD_BFF_PATH}");

    let result = Command::new(IGOR_PATH)
        .arg("-j=8")
        .arg(&options_arg)
        .arg("-v")
        .arg("--")
        .arg("Windows")
        .arg("Run")
        .creation_flags(CREATE_NO_WINDOW)
        .spawn();

    match result {
        Ok(_) => println!(
            "  Build + run launched for {}",
            yyp_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
        ),
        Err(e) => eprintln!("  Error: Failed to launch Igor.exe: {e}"),
    }
}
