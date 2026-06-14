use std::{
    fs,
    io::{BufReader, BufWriter},
};

#[derive(serde::Serialize, serde::Deserialize, Default)]
pub struct GmConfig {
    pub use_gm_beta: bool,
    pub all_sprites_export_yyp_path: Option<String>
}

pub fn get_or_create_config() -> Result<GmConfig, String> {
    let dir = dirs::data_dir().ok_or("data dir not found")?;

    let file_path = dir.join("config.json");

    if !file_path.exists() {
        let value = GmConfig::default();
        let file = fs::File::create(&file_path).map_err(|e| e.to_string())?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, &value).map_err(|e| e.to_string())?;
    }

    let file = fs::File::open(file_path).map_err(|e| e.to_string())?;
    let reader = BufReader::new(file);
    serde_json::from_reader(reader).map_err(|e| e.to_string())
}

/// You should use `get_or_create_config` first
pub fn write_config(value: &GmConfig) -> Result<(), String> {
    let dir = dirs::data_dir().ok_or("data dir not found")?;

    let file_path = dir.join("config.json");

    let file = fs::File::create(&file_path).map_err(|e| e.to_string())?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, &value).map_err(|e| e.to_string())
}
