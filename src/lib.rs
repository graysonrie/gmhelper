use std::{
    path::Path,
    sync::{Arc, atomic::AtomicBool},
    thread::{self, JoinHandle},
};

mod aseprite_exporter;
mod code_editor;
mod export_cache;
mod gm_config;
mod hot_reloader;
mod sprites;

const EXPORT_TAGS_SCRIPT: &str = include_str!("../lua/export_tags.lua");

pub fn export_all_sprites(
    path_to_sprites_dir: &Path,
    project_path: &Path,
    should_focus_gamemaker: bool,
    force_export: bool,
) -> Result<(), anyhow::Error> {
    aseprite_exporter::export_all_sprites(
        path_to_sprites_dir,
        project_path,
        should_focus_gamemaker,
        force_export,
    )
}

pub struct HotReloadTask {
    shutdown: Arc<AtomicBool>,
    thread: JoinHandle<()>,
}

impl HotReloadTask {
    pub fn shutdown_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.shutdown)
    }

    pub fn into_join_handle(self) -> JoinHandle<()> {
        self.thread
    }
}

pub fn start_hot_reload_task(path_to_yyp: std::path::PathBuf) -> HotReloadTask {
    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_for_thread = Arc::clone(&shutdown);

    let thread = thread::spawn(move || {
        if let Err(error) = hot_reloader::run_reload(path_to_yyp, &shutdown_for_thread) {
            eprintln!("Hot reload error: {error}");
        }
    });

    HotReloadTask { shutdown, thread }
}

pub use gm_config::{get_or_create_config, write_config};
