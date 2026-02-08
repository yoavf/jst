use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Default, Serialize, Deserialize)]
struct CacheFile {
    entries: HashMap<String, String>,
}

/// Simple disk-backed cache for accepted translations.
#[derive(Clone, Debug)]
pub struct PersistentCache {
    path: Option<PathBuf>,
    entries: HashMap<String, String>,
}

impl PersistentCache {
    /// Load cache from disk; tolerate any read/parse errors by falling back to empty cache.
    pub fn load() -> Self {
        let path = cache_file_path();
        let entries = path
            .as_ref()
            .and_then(|path| fs::read_to_string(path).ok())
            .and_then(|contents| serde_json::from_str::<CacheFile>(&contents).ok())
            .map(|file| file.entries)
            .unwrap_or_default();

        Self { path, entries }
    }

    pub fn get(&self, input: &str) -> Option<&String> {
        self.entries.get(input)
    }

    /// Insert and persist immediately; best-effort (logs errors, does not propagate).
    pub fn insert(&mut self, input: &str, command: &str) {
        self.entries.insert(input.to_string(), command.to_string());

        let Some(path) = &self.path else {
            return;
        };

        if let Some(parent) = path.parent() {
            if let Err(err) = fs::create_dir_all(parent) {
                eprintln!("jst: unable to create cache dir: {}", err);
                return;
            }
        }

        let data = CacheFile {
            entries: self.entries.clone(),
        };

        match serde_json::to_vec_pretty(&data) {
            Ok(bytes) => {
                let tmp_path = path.with_extension("tmp");
                if fs::write(&tmp_path, &bytes).is_ok() {
                    if let Err(err) = fs::rename(&tmp_path, path) {
                        eprintln!("jst: unable to finalize cache file: {}", err);
                    }
                } else if let Err(err) = fs::write(path, &bytes) {
                    eprintln!("jst: unable to write cache file: {}", err);
                }
            }
            Err(err) => eprintln!("jst: unable to serialize cache: {}", err),
        }
    }
}

fn cache_file_path() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".jst").join("cache.json"))
}
