use std::path::Path;

use crate::definition::{self, TextureDefinition};

const FILE_LOCATIONS: &str = "textures";

pub fn load_by_name(name: &str) -> Result<definition::TextureDefinition, String> {
    let path = Path::new(FILE_LOCATIONS).join(format!("{}.json", name));
    let file_content = std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;
    let mut ret:  TextureDefinition = serde_json::from_str(&file_content).map_err(|e| format!("Failed to parse JSON: {}", e))?;
    ret.name = name.to_string();
    Ok(ret)
}

pub fn save(def: &TextureDefinition) -> Result<(), String> {
    let path = Path::new(FILE_LOCATIONS).join(format!("{}.json", def.name));
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create directory: {}", e))?;
    }
    let json_content = serde_json::to_string(def).map_err(|e| format!("Failed to serialize to JSON: {}", e))?;
    std::fs::write(path, json_content).map_err(|e| format!("Failed to write file: {}", e))?;
    Ok(())
}
