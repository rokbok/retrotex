use std::{collections::{HashMap, hash_map::Entry}, sync::RwLock};

use rayon::prelude::*;

use crate::{file::FileId, prelude::*};
use crate::{definition::TextureDefinition, file::{DefinitionFile, FILE_EXTENSION, FILE_LOCATION}};

pub struct FileRegistry {
    files: HashMap<FileId, RwLock<DefinitionFile>>,
}

impl FileRegistry {
    pub fn read() -> Self {
        std::fs::create_dir_all(FILE_LOCATION)
            .expect("Failed to create texture storage directory");

        let mut loaded_files = std::fs::read_dir(FILE_LOCATION)
            .expect("Failed to read texture storage directory")
            .flatten()
            .par_bridge()
            .filter_map(|entry| {
                let path = entry.path();
                if !path.is_file() {
                    return None;
                }

                let has_correct_extension = path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| ext.eq_ignore_ascii_case(FILE_EXTENSION));

                if !has_correct_extension {
                    return None;
                }

                let name = path.file_stem()?.to_str()?;

                DefinitionFile::load_by_name(name)
                    .ok()
            })
            .collect::<Vec<DefinitionFile>>();

        loaded_files.sort_unstable_by(|a, b| a.name().cmp(b.name()));

        let mut files = HashMap::<FileId, RwLock<DefinitionFile>>::with_capacity(loaded_files.len());
        for file in loaded_files {
            let id = file.id();
            let name = file.name().to_string();

            match files.entry(id) {
                Entry::Vacant(entry) => {
                    entry.insert(RwLock::new(file));
                }
                Entry::Occupied(existing) => {
                    error!(
                        "Duplicate texture id {} detected; keeping '{}' and skipping '{}', resolved by filename order",
                        id,
                        existing.get().read().unwrap().name(),
                        name,
                    );
                }
            }
        }

        Self { files }
    }

    pub fn id_by_name(&self, name: &str) -> Option<u128> {
        self.files.values()
            .find(| file | file.read().unwrap().name() == name)
            .map(| file | file.read().unwrap().id() )
    }

    pub fn file_by_id(&self, id: u128) -> Option<&RwLock<DefinitionFile>> {
        self.files.get(&id)
    }

    pub fn create(&mut self, name: &str, def: TextureDefinition) -> u128 {
        let file = DefinitionFile::new_with_def(name.to_string(), def);
        let id = file.id();
        self.files.insert(id, RwLock::new(file));
        id
    }

    pub fn files_sorted(&self) -> Vec<(u128, String)> {
        let mut files = self
            .files
            .values()
            .map(|file| {
                let file = file.read().unwrap();
                (file.id(), file.name().to_string())
            })
            .collect::<Vec<_>>();

        files.sort_unstable_by(|a, b| a.1.cmp(&b.1));
        files
    }
}
