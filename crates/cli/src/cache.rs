use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::hash::{Hash, Hasher};
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

/// Prompt history — stores natural language inputs that were executed.
#[derive(Clone, Debug)]
pub struct PromptHistory {
    path: Option<PathBuf>,
    entries: Vec<String>,
}

const MAX_HISTORY: usize = 500;

impl PromptHistory {
    pub fn load() -> Self {
        let path = dirs::home_dir().map(|home| home.join(".jst").join("history.json"));
        let entries = path
            .as_ref()
            .and_then(|p| fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str::<Vec<String>>(&s).ok())
            .unwrap_or_default();
        Self { path, entries }
    }

    pub fn entries(&self) -> &[String] {
        &self.entries
    }

    pub fn push(&mut self, input: &str) {
        let trimmed = input.trim().to_string();
        if trimmed.is_empty() {
            return;
        }
        // Remove duplicate if it already exists so it moves to the end
        self.entries.retain(|e| e != &trimmed);
        self.entries.push(trimmed);
        if self.entries.len() > MAX_HISTORY {
            self.entries.drain(..self.entries.len() - MAX_HISTORY);
        }
        self.save();
    }

    fn save(&self) {
        let Some(path) = &self.path else { return };
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(bytes) = serde_json::to_vec_pretty(&self.entries) {
            let tmp = path.with_extension("tmp");
            if fs::write(&tmp, &bytes).is_ok() {
                let _ = fs::rename(&tmp, path);
            }
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct LastCommandFile {
    last_command: String,
}

pub fn load_last_command_for_current_shell_session() -> Option<String> {
    let path = last_command_file_path_for_current_shell_session()?;
    let contents = fs::read_to_string(path).ok()?;
    let file = serde_json::from_str::<LastCommandFile>(&contents).ok()?;
    let cmd = file.last_command.trim();
    if cmd.is_empty() {
        None
    } else {
        Some(cmd.to_string())
    }
}

pub fn save_last_command_for_current_shell_session(command: &str) {
    let Some(path) = last_command_file_path_for_current_shell_session() else {
        return;
    };

    if let Some(parent) = path.parent() {
        if let Err(err) = fs::create_dir_all(parent) {
            eprintln!("jst: unable to create session cache dir: {}", err);
            return;
        }
    }

    let data = LastCommandFile {
        last_command: command.to_string(),
    };

    match serde_json::to_vec_pretty(&data) {
        Ok(bytes) => {
            let tmp_path = path.with_extension("tmp");
            if fs::write(&tmp_path, &bytes).is_ok() {
                if let Err(err) = fs::rename(&tmp_path, path) {
                    eprintln!("jst: unable to finalize session cache file: {}", err);
                }
            } else if let Err(err) = fs::write(path, &bytes) {
                eprintln!("jst: unable to write session cache file: {}", err);
            }
        }
        Err(err) => eprintln!("jst: unable to serialize session cache: {}", err),
    }
}

fn last_command_file_path_for_current_shell_session() -> Option<PathBuf> {
    let session_id = current_shell_session_id();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    session_id.hash(&mut hasher);
    let session_hash = format!("{:016x}", hasher.finish());

    dirs::home_dir().map(|home| {
        home.join(".jst")
            .join("session")
            .join(format!("last_command_{}.json", session_hash))
    })
}

fn current_shell_session_id() -> String {
    for key in [
        "JST_SHELL_SESSION_ID",
        "TERM_SESSION_ID",
        "ITERM_SESSION_ID",
        "WT_SESSION",
        "TMUX_PANE",
        "TMUX",
        "STY",
        "PPID",
    ] {
        if let Ok(value) = std::env::var(key) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return format!("{}={}", key, trimmed);
            }
        }
    }

    "fallback-default-session".to_string()
}
