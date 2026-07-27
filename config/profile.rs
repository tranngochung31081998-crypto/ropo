use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// User profile management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub home_dir: PathBuf,
    pub data_dir: PathBuf,
    pub config_dir: PathBuf,
}

impl Profile {
    pub fn new(name: &str) -> Self {
        let base = dirs_data_dir();
        Self {
            name: name.to_string(),
            home_dir: base.join(name),
            data_dir: base.join(name).join("data"),
            config_dir: base.join(name).join("config"),
        }
    }
}

fn dirs_data_dir() -> PathBuf {
    std::env::var("CULI_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::data_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("culi")
        })
}
