use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
};

use crate::{
    operations,
    util::{self, PathStringUtil},
};

const FAMITRACKER_SILENCE_START: f64 = 0.084;
const FAMITRACKER_SILENCE_END: f64 = 0.1;

pub struct GameMusicExportOptions {
    pub trim_start_secs: f64,
    pub trim_end_secs: f64,
}
impl GameMusicExportOptions {
    pub fn famitracker_defaults() -> Self {
        Self {
            trim_start_secs: FAMITRACKER_SILENCE_START,
            trim_end_secs: FAMITRACKER_SILENCE_END,
        }
    }
}

pub enum Mp4LoopOption {
    SetValue(u32),
    /// Example: have shorter songs get looped longer (3x) other songs will just get looped 2x
    BasedOffLength,
}
pub struct Mp4ExportOptions {
    pub video_image_path: String,
    pub game_title: String,
    pub loops: Mp4LoopOption,
    pub fade_duration_secs: f64,
    pub lead_in_silence_secs: f64,
}
impl Mp4ExportOptions {
    pub fn defaults(video_image_path: &str, game_title: &str) -> Self {
        let video_image_path = video_image_path.to_string();
        let game_title = game_title.to_string();
        Self {
            video_image_path,
            game_title,
            fade_duration_secs: 8.,
            lead_in_silence_secs: 0.06,
            loops: Mp4LoopOption::BasedOffLength,
        }
    }
}

pub struct MusicExportResult {
    pub num_files_exported: usize,
}

/// game_title does not need to include 'OST' in it
pub fn export_as_mp4_files(
    project_folder_path: &Path,
    options: &GameMusicExportOptions,
    mp4_options: &Mp4ExportOptions,
) -> Result<MusicExportResult, Box<dyn Error>> {
    let music_folder_path = get_music_folder_path(project_folder_path)?;

    let output_music_folder_path = music_folder_path.join("Mp4GameMusic");
    fs::create_dir_all(&output_music_folder_path)?;

    let music_files: Vec<PathBuf> = music_folder_path
        .read_dir()?
        .flatten()
        .map(|f| f.path())
        .filter(|path| path.extension().unwrap_or_default() == "wav")
        .collect();
    let mut num_files_exported = 0;

    let Mp4ExportOptions {
        video_image_path,
        game_title,
        loops,
        fade_duration_secs,
        lead_in_silence_secs,
    } = mp4_options;

    for (i, music_file) in music_files.iter().enumerate() {
        let input_path_filename = music_file.unwrap_filename();
        let input_path = music_file.into_string(); // The full input path

        // Ex: 3 becomes 03 and 12 just becomes 12
        let output_filename_prefix = format!("{:02}. ", i);
        let output_end = format!(" ({game_title} OST).mp4");

        let output_mp4_filename = output_filename_prefix
            + &util::convert_to_pascal_case(&input_path_filename.replace(".wav", ""))
            + &output_end;

        // Clean the wav file
        let temp_trimmed_wav_path = output_music_folder_path
            .join(format!(
                "{}.trimmed.wav",
                input_path_filename.trim_end_matches(".wav")
            ))
            .into_string();

        let trim_start_secs = options.trim_start_secs;
        let trim_end_secs = options.trim_end_secs;

        let trim_result = operations::trim_wav(
            &input_path,
            &temp_trimmed_wav_path,
            trim_start_secs,
            trim_end_secs,
        )?;
        // Done trimming. Export production ver:

        let temp_prod_wav_path = output_music_folder_path
            .join(format!(
                "{}.prod.wav",
                input_path_filename.trim_end_matches(".wav")
            ))
            .into_string();

        let loop_num = match loops {
            Mp4LoopOption::SetValue(val) => val,
            Mp4LoopOption::BasedOffLength => {
                if trim_result.new_duration_secs < 30. {
                    &3
                } else {
                    &2
                }
            }
        };

        operations::export_production_wav_file(
            &temp_trimmed_wav_path,
            &temp_prod_wav_path,
            *loop_num,
            *fade_duration_secs,
            *lead_in_silence_secs,
        )?;
        // Delete the temp trimmed wav
        fs::remove_file(&temp_trimmed_wav_path)?;

        let output_mp4_path = output_music_folder_path
            .join(output_mp4_filename)
            .into_string();
        operations::export_production_mp4(&temp_prod_wav_path, &output_mp4_path, video_image_path)?;
        // Delete the temp prod wav:
        fs::remove_file(&temp_prod_wav_path)?;

        num_files_exported += 1;
    }

    Ok(MusicExportResult { num_files_exported })
}

/// goes inside the 'music' folder at `project_folder_path` and takes all of the wav files in there and exports them to be usable in GameMaker:
/// ex: song1.wav --> sndSong1.ogg
pub fn export_as_game_music(
    project_folder_path: &Path,
    options: &GameMusicExportOptions,
) -> Result<MusicExportResult, Box<dyn Error>> {
    let music_folder_path = get_music_folder_path(project_folder_path)?;

    let output_music_folder_path = music_folder_path.join("GameMusic");
    fs::create_dir_all(&output_music_folder_path)?;

    let music_files: Vec<PathBuf> = music_folder_path
        .read_dir()?
        .flatten()
        .map(|f| f.path())
        .filter(|path| path.extension().unwrap_or_default() == "wav")
        .collect();

    let mut num_files_exported = 0;
    for music_file in music_files {
        let input_path_filename = music_file.unwrap_filename();
        let input_path = music_file.into_string(); // The full input path

        let output_filename = "snd".to_string()
            + &util::convert_to_pascal_case(&input_path_filename.replace(".wav", ""))
            + ".ogg";

        let output_ogg_path = output_music_folder_path.join(output_filename).into_string();
        let temp_trimmed_wav_path = output_music_folder_path
            .join(format!(
                "{}.trimmed.wav",
                input_path_filename.trim_end_matches(".wav")
            ))
            .into_string();

        let trim_start_secs = options.trim_start_secs;
        let trim_end_secs = options.trim_end_secs;

        operations::trim_wav(
            &input_path,
            &temp_trimmed_wav_path,
            trim_start_secs,
            trim_end_secs,
        )?;
        operations::wav_to_ogg(&temp_trimmed_wav_path, &output_ogg_path)?;
        fs::remove_file(&temp_trimmed_wav_path)?;
        num_files_exported += 1;
    }

    Ok(MusicExportResult { num_files_exported })
}

fn get_music_folder_path(project_folder_path: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let dirs = project_folder_path.read_dir()?;

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

    Err("Music folder was not found".into())
}
