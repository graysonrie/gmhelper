/// Trims a WAV file from the start and end
/// # Arguments
/// * `input_path` - The path to the input WAV file
/// * `output_path` - The path to the output WAV file
/// * `trim_start_secs` - The number of seconds to trim from the start
/// * `trim_end_secs` - The number of seconds to trim from the end
/// # Returns
/// * `Ok(())` - If the file was trimmed successfully
/// * `Err(e)` - If the file was not trimmed successfully
pub fn trim_wav(
    input_path: &str,
    output_path: &str,
    trim_start_secs: f64,
    trim_end_secs: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    let reader = hound::WavReader::open(input_path)?;
    let spec = reader.spec();

    let samples_per_second = spec.sample_rate as f64 * spec.channels as f64;
    let all_samples: Vec<i32> = reader.into_samples::<i32>().collect::<Result<_, _>>()?;

    let skip_start = (trim_start_secs * samples_per_second) as usize;
    let skip_end = (trim_end_secs * samples_per_second) as usize;

    let total = all_samples.len();
    if total <= skip_start + skip_end {
        return Err(format!(
            "File is too short to trim {trim_start_secs}s from start and {trim_end_secs}s from end"
        )
        .into());
    }

    let trimmed = &all_samples[skip_start..total - skip_end];

    let mut writer = hound::WavWriter::create(output_path, spec)?;
    for &sample in trimmed {
        writer.write_sample(sample)?;
    }
    writer.finalize()?;

    Ok(())
}

pub fn wav_to_ogg(input_path: &str, output_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let output = std::process::Command::new("ffmpeg")
        .args([
            "-y",
            "-i",
            input_path,
            "-c:a",
            "libvorbis",
            "-q:a",
            "5",
            output_path,
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("ffmpeg failed: {stderr}").into());
    }

    Ok(())
}

/// Will export a new WAV file with the given number of loops and fade duration.
/// Applies a subtle reverb and gentle EQ (slight bass warmth, mild high-end rolloff).
pub fn export_production_wav_file(
    seamlessly_looping_wav_path: &str,
    output_wav_path: &str,
    loops: u32,
    fade_duration_secs: f64,
    lead_in_silence_secs: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    let probe = std::process::Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "csv=p=0",
            seamlessly_looping_wav_path,
        ])
        .output()?;

    if !probe.status.success() {
        let stderr = String::from_utf8_lossy(&probe.stderr);
        return Err(format!("ffprobe failed: {stderr}").into());
    }

    let duration_str = String::from_utf8_lossy(&probe.stdout);
    let input_duration: f64 = duration_str.trim().parse()?;

    if fade_duration_secs > input_duration {
        return Err(format!(
            "Fade duration ({fade_duration_secs}s) is longer than a single loop ({input_duration}s)"
        ).into());
    }

    let delay_ms = (lead_in_silence_secs * 1000.0) as u32;
    // N full loops + 1 extra loop for the fade-out, shifted by lead-in silence
    let fade_start = lead_in_silence_secs + input_duration * loops as f64;
    let final_duration = fade_start + fade_duration_secs;

    let effects_filter = format!(
        "adelay={delay_ms}|{delay_ms},\
         aecho=0.8:0.3:25:0.15,\
         highpass=f=60,\
         equalizer=f=250:t=q:w=1.5:g=-2,\
         equalizer=f=3000:t=q:w=1.5:g=2,\
         equalizer=f=10000:t=h:w=2000:g=1,\
         afade=t=out:st={fade_start}:d={fade_duration_secs},\
         atrim=end={final_duration}"
    );

    // N full plays + 1 extra for the fade
    let stream_loops = loops.to_string();

    // Pass 1: apply effects, detect peak volume
    let detect_filter = format!("{effects_filter},volumedetect");
    let detect = std::process::Command::new("ffmpeg")
        .args([
            "-y",
            "-stream_loop",
            &stream_loops,
            "-i",
            seamlessly_looping_wav_path,
            "-af",
            &detect_filter,
            "-f",
            "null",
            "-",
        ])
        .output()?;

    if !detect.status.success() {
        let stderr = String::from_utf8_lossy(&detect.stderr);
        return Err(format!("ffmpeg volumedetect failed: {stderr}").into());
    }

    let detect_stderr = String::from_utf8_lossy(&detect.stderr);
    let max_volume = detect_stderr
        .lines()
        .find(|l| l.contains("max_volume:"))
        .and_then(|l| {
            l.split("max_volume:")
                .nth(1)?
                .trim()
                .strip_suffix("dB")?
                .trim()
                .parse::<f64>()
                .ok()
        })
        .ok_or("failed to parse max_volume from ffmpeg output")?;

    // Uniform gain to bring peak to -1 dBFS (1dB headroom)
    let gain = 2.0 - max_volume;

    // Pass 2: apply effects + uniform gain
    let final_filter = format!("{effects_filter},volume={gain}dB");
    let output = std::process::Command::new("ffmpeg")
        .args([
            "-y",
            "-stream_loop",
            &stream_loops,
            "-i",
            seamlessly_looping_wav_path,
            "-af",
            &final_filter,
            output_wav_path,
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("ffmpeg failed: {stderr}").into());
    }

    Ok(())
}

/// Will export a new MP4 file with the given production WAV file and video image.
pub fn export_production_mp4(
    production_wav_path: &str,
    output_mp4_path: &str,
    video_image_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let probe = std::process::Command::new("ffprobe")
        .args([
            "-v", "error",
            "-show_entries", "format=duration",
            "-of", "csv=p=0",
            production_wav_path,
        ])
        .output()?;

    if !probe.status.success() {
        let stderr = String::from_utf8_lossy(&probe.stderr);
        return Err(format!("ffprobe failed: {stderr}").into());
    }

    let duration = String::from_utf8_lossy(&probe.stdout).trim().to_string();

    let output = std::process::Command::new("ffmpeg")
        .args([
            "-y",
            "-loop", "1",
            "-i", video_image_path,
            "-i", production_wav_path,
            "-c:v", "libx264",
            "-tune", "stillimage",
            "-c:a", "aac",
            "-b:a", "192k",
            "-pix_fmt", "yuv420p",
            "-t", &duration,
            output_mp4_path,
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("ffmpeg failed: {stderr}").into());
    }

    Ok(())
}