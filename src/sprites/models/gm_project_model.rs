use serde::{Deserialize, Serialize};

/// A single folder entry in the `.yyp` `Folders` array.
/// Used only for serializing new folder entries to insert into the project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GMFolder {
    #[serde(rename = "$GMFolder")]
    pub gm_folder: String,

    #[serde(rename = "%Name")]
    pub name_field: String,

    #[serde(rename = "folderPath")]
    pub folder_path: String,

    pub name: String,

    #[serde(rename = "resourceType")]
    pub resource_type: String,

    #[serde(rename = "resourceVersion")]
    pub resource_version: String,
}

impl GMFolder {
    /// Create a new GMFolder entry for the `.yyp` Folders array.
    ///
    /// `name` is the display name (e.g. "Enemies").
    /// `folder_path` is the full folder path (e.g. "folders/Sprites/Enemies.yy").
    pub fn new(name: &str, folder_path: &str) -> Self {
        Self {
            gm_folder: String::new(),
            name_field: name.to_string(),
            folder_path: folder_path.to_string(),
            name: name.to_string(),
            resource_type: "GMFolder".to_string(),
            resource_version: "2.0".to_string(),
        }
    }
}
