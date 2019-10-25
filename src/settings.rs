use anyhow::{Context, Result};
use lazy_static::lazy_static;
use log::info;
use serde::{Deserialize, Serialize};
use std::{
    fs::OpenOptions,
    io::{Read, Write},
    path::PathBuf,
};

static CONFIG_PATH: &'static str = "config";
static CONFIG_FILE: &'static str = "config/Settings.ron";

lazy_static! {
    pub static ref SETTINGS: Settings = {
        load_settings(CONFIG_FILE).expect(&format!(
            "Failed to load settings from file {}",
            CONFIG_FILE
        ))
    };
}

fn load_settings(path: impl Into<PathBuf>) -> Result<Settings> {
    // Read config
    info!("Reading config...");
    let path = path.into();
    let config = if path.is_file() {
        let mut config_file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .context(format!("Failed to open settings file {}", path.display()))?;
        let mut buf = String::new();
        config_file
            .read_to_string(&mut buf)
            .context(format!("Failed to read settings file {}", path.display()))?;
        ron::de::from_str(&buf)
            .context(format!("Failed to parse settings file {}", path.display()))?
    } else {
        std::fs::create_dir_all(&CONFIG_PATH)?;
        Settings::default()
    };

    info!("Writing config...");
    // Write config
    let mut config_file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(&path)
        .context(format!("Failed to open settings file {}", path.display()))?;
    let string = ron::ser::to_string(&config).context("Failed to serialize settings")?;
    config_file
        .write(string.as_bytes())
        .context(format!("Failed to write settings file {}", path.display()))?;
    Ok(config)
}

/// Settings of the game
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct Settings {
    pub window_size: (u32, u32),
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            window_size: (1600, 900),
        }
    }
}
