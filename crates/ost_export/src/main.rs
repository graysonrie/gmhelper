use std::path::Path;
mod operations;
use operations::*;
mod api;
mod util;

fn main() {
    let input = "C:\\Users\\grays\\Downloads\\test.wav";
    let output = Path::new(input).with_extension("trimmed.wav");
    let output = output.to_str().expect("invalid path");

    trim_wav(input, output, 0.084, 0.1).expect("Failed to trim file");

    let output_production = Path::new(input).with_extension("production.wav");
    let output_production = output_production.to_str().expect("invalid path");
    export_production_wav_file(output, output_production, 2, 8.5, 0.1)
        .expect("Failed to export production file");

    let output_mp4 = Path::new(input).with_extension("production.mp4");
    let output_mp4 = output_mp4.to_str().expect("invalid path");
    let image_path = r"C:\\Users\\grays\\Pictures\\Screenshots\\Screenshot 2025-03-23 231208.png";
    export_production_mp4(output_production, output_mp4, image_path)
        .expect("Failed to export production MP4 file");

    let output_ogg = &output.replace(".wav", ".ogg");
    wav_to_ogg(output, output_ogg).expect("Failed to convert to OGG");
}
