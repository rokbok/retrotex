use std::path::Path;

use crate::definition::{self, TextureDefinition};

const FILE_LOCATIONS: &str = "textures";
const UNDO_LIMIT: usize = 1000;

#[allow(unused_imports)]
use log::{debug, error, log_enabled, info, warn, trace};

pub struct LoadSaveUndo {
    loaded: Option<String>,
    undo_stack: Vec<String>,
    redo_index: usize,
}

impl LoadSaveUndo {
    pub fn new() -> Self {
        Self {
            loaded: None,
            undo_stack: Vec::new(),
            redo_index: 0,
        }
    }

    pub fn load_by_name_or_create_default(&mut self, name: &str) -> TextureDefinition {
        match self.load_by_name(name) {
            Ok(def) => {
                info!("Loaded texture definition: {}", def.name);
                def
            },
            Err(e) => {
                warn!("Failed to load texture definition, creating default: {}", e);
                let mut def = TextureDefinition::default();
                def.name = name.to_string();
                self.loaded = Some(name.to_string());
                self.save(&def).expect("Failed to save default texture definition");
                def
            },
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
        self.redo_index = 1;
        Ok(ret)
    }

    pub fn save(&mut self, def: &TextureDefinition) -> Result<(), String> {
        assert!(self.loaded.as_ref().map(| ld | *ld == def.name).unwrap_or(false));

        let json_content = serde_json::to_string(def).map_err(|e| format!("Failed to serialize to JSON: {}", e))?;
        if self.redo_index < self.undo_stack.len() {
            self.undo_stack.truncate(self.redo_index);
        }
        self.undo_stack.push(json_content);
        self.redo_index = self.undo_stack.len();
        while self.undo_stack.len() > UNDO_LIMIT {
            self.undo_stack.remove(0);
            self.redo_index -= 1;
        }

        let path = Path::new(FILE_LOCATIONS).join(format!("{}.json", def.name));
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create directory: {}", e))?;
        }
        std::fs::write(path, self.undo_stack.last().unwrap()).map_err(|e| format!("Failed to write file: {}", e))?;
        Ok(())
    }

    pub fn undo(&mut self) -> Option<TextureDefinition> {
        if self.redo_index <= 1 {
            return None;
        }
        self.redo_index -= 1;
        let last_content = self.undo_stack[self.redo_index - 1].as_str();
        let mut ret: TextureDefinition = serde_json::from_str(last_content).ok().expect("How did we get invalid JSON in the undo stack?");
        ret.name = self.loaded.as_ref().unwrap().to_string();
        Some(ret)
    }
    
    pub fn redo(&mut self) -> Option<TextureDefinition> {
        if self.redo_index >= self.undo_stack.len() {
            return None;
        }
        let last_content = self.undo_stack[self.redo_index].as_str();
        let mut ret: TextureDefinition = serde_json::from_str(last_content).ok().expect("How did we get invalid JSON in the undo stack?");
        ret.name = self.loaded.as_ref().unwrap().to_string();
        self.redo_index += 1;
        Some(ret)
    }
}
