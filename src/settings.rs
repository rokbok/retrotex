use std::{fs::File, io::{BufReader, BufWriter}, path::Path};

use serde::{Deserialize, Serialize};

use crate::prelude::*;

const SETTINGS_PATH: &str = "retrotex.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub last_opened_id: u128,
    #[serde(skip)]
    pub last_saved_hash: u64,
}


impl Default for Settings {
    fn default() -> Self {
        Self {
            last_opened_id: 0,
            last_saved_hash: 0,
        }
    }
}

impl Settings {
    fn current_hash(&self) -> u64 {
        crate::util::single_hash(&self.last_opened_id)
    }

    fn from_loaded(mut settings: Self) -> Self {
        settings.last_saved_hash = settings.current_hash();
        settings
    }

    pub fn load() -> Self {
        let path = Path::new(SETTINGS_PATH);
        if !path.exists() {
            return Self::from_loaded(Self::default());
        }

        let reader = match File::open(path).map(BufReader::new) {
            Ok(reader) => reader,
            Err(e) => {
                info!("Failed to open settings file '{}': {} -- using defaults", SETTINGS_PATH, e);
                return Self::from_loaded(Self::default());
            }
        };

        match serde_json::from_reader(reader) {
            Ok(settings) => Self::from_loaded(settings),
            Err(e) => {
                warn!("Failed to parse settings file '{}': {}", SETTINGS_PATH, e);
                Self::from_loaded(Self::default())
            }
        }
    }

    fn save(&self) -> Result<(), String> {
        let writer = BufWriter::new(
            File::create(SETTINGS_PATH)
                .map_err(|e| format!("Failed to create settings file '{}': {}", SETTINGS_PATH, e))?,
        );
        serde_json::to_writer_pretty(writer, self)
            .map_err(|e| format!("Failed to write settings file '{}': {}", SETTINGS_PATH, e))
    }

    pub fn save_if_changed(&mut self) {
        let current_hash = self.current_hash();
        if current_hash == self.last_saved_hash {
            return;
        }

        if let Err(e) = self.save() {
            error!("{}", e);
            return;
        }

        self.last_saved_hash = current_hash;
    }
}