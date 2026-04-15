use crate::theme::Theme;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub theme: Theme,
    #[serde(default)]
    pub recent_files: Vec<PathBuf>,
    /// When true, no history entries are recorded.
    #[serde(default)]
    pub private_mode: bool,
}

impl Settings {
    pub fn load() -> Self {
        let Some(path) = config_path() else {
            return Self::default();
        };
        match std::fs::read_to_string(&path) {
            Ok(text) => toml::from_str(&text).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) {
        let Some(path) = config_path() else { return };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(text) = toml::to_string_pretty(self) {
            let _ = std::fs::write(&path, text);
        }
    }

    pub fn push_recent_file(&mut self, path: PathBuf) {
        self.recent_files.retain(|p| p != &path);
        self.recent_files.insert(0, path);
        self.recent_files.truncate(10);
    }
}

fn config_path() -> Option<PathBuf> {
    ProjectDirs::from("com", "basie64", "basie-64").map(|d| d.config_dir().join("config.toml"))
}
