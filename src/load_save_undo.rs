use std::path::Path;

use crate::definition::{self, TextureDefinition};

const FILE_LOCATIONS: &str = "textures";

#[allow(unused_imports)]
use log::{debug, error, log_enabled, info, warn, trace};

pub struct LoadSaveUndo {
    loaded: Option<String>,
    undo_stack: Vec<String>,
}

impl LoadSaveUndo {
    pub fn new() -> Self {
        Self {
            loaded: None,
            undo_stack: Vec::new(),
        }
    }

    pub fn load_by_name(&mut self, name: &str) -> Result<definition::TextureDefinition, String> {
        let path = Path::new(FILE_LOCATIONS).join(format!("{}.json", name));
        let file_content = std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;
        let mut ret:  TextureDefinition = serde_json::from_str(&file_content).map_err(|e| format!("Failed to parse JSON: {}", e))?;
        ret.name = name.to_string();
        self.loaded = Some(name.to_string());
        self.undo_stack.clear();
        self.undo_stack.push(file_content);
        Ok(ret)
    }

    pub fn save(&mut self, def: &TextureDefinition) -> Result<(), String> {
        assert!(self.loaded.as_ref().map(| ld | *ld == def.name).unwrap_or(false));

        let json_content = serde_json::to_string(def).map_err(|e| format!("Failed to serialize to JSON: {}", e))?;
        self.undo_stack.push(json_content);

        let path = Path::new(FILE_LOCATIONS).join(format!("{}.json", def.name));
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create directory: {}", e))?;
        }
        std::fs::write(path, self.undo_stack.last().unwrap()).map_err(|e| format!("Failed to write file: {}", e))?;
        Ok(())
    }

    pub fn undo(&mut self) -> Option<TextureDefinition> {
        if self.undo_stack.len() <= 1 {
            return None;
        }
        self.undo_stack.pop();
        let last_content = self.undo_stack.last().unwrap();
        let mut ret: TextureDefinition = serde_json::from_str(last_content).ok().expect("How did we get invalid JSON in the undo stack?");
        ret.name = self.loaded.as_ref().unwrap().to_string();
        Some(ret)
    }
}
