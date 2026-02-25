use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{operations, util};

const FAMITRACKER_SILENCE_START:f64 = 0.084;
const FAMITRACKER_SILENCE_END:f64 = 0.1;

pub struct GameMusicExportOptions {
  pub trim_start_secs:f64,
  pub trim_end_secs:f64,
}
impl GameMusicExportOptions {
  pub fn famitracker_defaults()->Self{
    Self {
      trim_start_secs: FAMITRACKER_SILENCE_START,
      trim_end_secs: FAMITRACKER_SILENCE_END,
    }
  }
}

/// goes inside the 'music' folder at `project_folder_path` and takes all of the wav files in there and exports them to be usable in GameMaker:
/// ex: song1.wav --> sndSong1.ogg
pub fn export_as_game_music(project_folder_path: &Path, options: &GameMusicExportOptions) -> Result<(), String> {
    let music_folder_path = get_music_folder_path(project_folder_path)?;

    let output_music_folder_path = music_folder_path.join("GameMusic");
    fs::create_dir(&output_music_folder_path).map_err(|e| e.to_string())?;

    let music_files: Vec<PathBuf> = music_folder_path
        .read_dir()
        .map_err(|e| e.to_string())?
        .flatten()
        .map(|f| f.path())
        .filter(|path| path.ends_with("wav"))
        .collect();

    for music_file in music_files {
      let input_path_filename = music_file.file_name().ok_or("No filename".to_string())?.to_string_lossy().to_string(); // Ex: song.wav
      let input_path = music_file.to_string_lossy().to_string(); // The full input path

      let output_filename = "snd".to_string() + &util::convert_to_pascal_case(&input_path_filename.replace(".wav", "")) + ".ogg";

      let output_path = output_music_folder_path.join(output_filename).to_string_lossy().to_string() ;
      let trim_start_secs= options.trim_start_secs;
      let trim_end_secs= options.trim_end_secs;

      operations::trim_wav(&input_path, &output_path, trim_start_secs, trim_end_secs).map_err(|e|e.to_string())?;
    }

    Ok(())
}

fn get_music_folder_path(project_folder_path: &Path) -> Result<PathBuf, String> {
    let dirs = project_folder_path.read_dir().map_err(|e| e.to_string())?;

    for dir in dirs.into_iter().flatten() {
        let entry_path = dir.path();
        if entry_path.is_dir() {
            let fname = dir.file_name();
            if fname == "music" || fname == "Music" {
                // Found the music folder
                return Ok(entry_path);
            }
        }
    }

    Err("Music folder was not found".to_string())
}
