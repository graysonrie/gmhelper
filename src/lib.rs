use std::path::Path;
use tokio::task::JoinHandle;

mod aseprite_exporter;
mod code_editor;
mod gm_config;
mod history;
mod hot_reloader;
mod sprites;

const EXPORT_TAGS_SCRIPT: &str = include_str!("../lua/export_tags.lua");

pub fn export_all_sprites(
    path_to_sprites_dir: &Path,
    project_path: &Path,
    should_focus_gamemaker: bool,
) -> Result<(), anyhow::Error> {
    aseprite_exporter::export_all_sprites(path_to_sprites_dir, project_path, should_focus_gamemaker)
}

pub fn run_hot_reload_task(path_to_yyp: std::path::PathBuf) -> JoinHandle<()> {
    tokio::task::spawn(async move {
        hot_reloader::run_reload(path_to_yyp);
    })
}
