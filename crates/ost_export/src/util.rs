use std::path::PathBuf;

pub fn convert_to_pascal_case(input: &str) -> String {
    input
        .split_whitespace()
        .map(|word| word.chars().next().unwrap().to_uppercase().to_string() + &word[1..])
        .collect::<String>()
}

pub trait PathStringUtil {
    fn into_string(&self) -> String;
    /// Warning!: will panic if the Path does not have a file name for some reason
    fn unwrap_filename(&self) -> String;
}

impl PathStringUtil for PathBuf {
    fn unwrap_filename(&self) -> String {
        self.file_name()
            .expect("No file name")
            .to_string_lossy()
            .to_string()
    }

    fn into_string(&self) -> String {
        self.to_string_lossy().to_string()
    }
}
