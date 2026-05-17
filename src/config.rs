use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct GlobalConfig {
    #[serde(default)]
    pub midi: MidiConfig,
    #[serde(default)]
    pub studio: StudioConfig,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct MidiConfig {
    pub output_port: Option<usize>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct StudioConfig {
    #[serde(default)]
    pub note_keyboard: NoteKeyboardConfig,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct NoteKeyboardConfig {
    pub base_octave: Option<i32>,
    pub octave_down: Option<String>,
    pub octave_up: Option<String>,
    #[serde(default)]
    pub keys: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub enum ConfigLoadStatus {
    Loaded(PathBuf),
    NotFound(PathBuf),
    ReadError { path: PathBuf, error: String },
    ParseError { path: PathBuf, error: String },
    PathUnavailable,
}

#[derive(Debug, Clone)]
pub struct LoadedGlobalConfig {
    pub config: GlobalConfig,
    pub status: ConfigLoadStatus,
}

impl LoadedGlobalConfig {
    pub fn status_message(&self) -> String {
        match &self.status {
            ConfigLoadStatus::Loaded(path) => format!("Config: {}", path.display()),
            ConfigLoadStatus::NotFound(path) => {
                format!("Config: not found (default) [{}]", path.display())
            }
            ConfigLoadStatus::ReadError { path, error } => {
                format!(
                    "Config: read error (default) [{}] {}",
                    path.display(),
                    error
                )
            }
            ConfigLoadStatus::ParseError { path, error } => {
                format!("Config: invalid (default) [{}] {}", path.display(), error)
            }
            ConfigLoadStatus::PathUnavailable => "Config: path unavailable (default)".to_string(),
        }
    }
}

pub fn load_global_config() -> LoadedGlobalConfig {
    let Some(path) = default_config_path() else {
        return LoadedGlobalConfig {
            config: GlobalConfig::default(),
            status: ConfigLoadStatus::PathUnavailable,
        };
    };

    if !path.exists() {
        return LoadedGlobalConfig {
            config: GlobalConfig::default(),
            status: ConfigLoadStatus::NotFound(path),
        };
    }

    let content = match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(err) => {
            eprintln!("Warning: failed to read config {}: {}", path.display(), err);
            return LoadedGlobalConfig {
                config: GlobalConfig::default(),
                status: ConfigLoadStatus::ReadError {
                    path,
                    error: err.to_string(),
                },
            };
        }
    };

    match toml::from_str::<GlobalConfig>(&content) {
        Ok(config) => LoadedGlobalConfig {
            config,
            status: ConfigLoadStatus::Loaded(path),
        },
        Err(err) => {
            eprintln!("Warning: invalid config {}: {}", path.display(), err);
            LoadedGlobalConfig {
                config: GlobalConfig::default(),
                status: ConfigLoadStatus::ParseError {
                    path,
                    error: err.to_string(),
                },
            }
        }
    }
}

fn default_config_path() -> Option<PathBuf> {
    let mut home = std::env::var_os("HOME").map(PathBuf::from)?;
    home.push(".config");
    home.push("loom");
    home.push("loom.toml");
    Some(home)
}
