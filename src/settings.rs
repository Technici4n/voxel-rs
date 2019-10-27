use anyhow::{Context, Result};
use lazy_static::lazy_static;
use log::info;
use serde::{Deserialize, Serialize};
use std::{
    fs::OpenOptions,
    io::{Read, Write},
    path::Path,
    sync::RwLock,
};

// TODO: don't make this static

static SETTINGS_PATH: &'static str = "config";
static SETTINGS_FILE: &'static str = "config/Settings.ron";

lazy_static! {
    pub static ref SETTINGS: RwLock<Settings> = {
        RwLock::new(load_settings(SETTINGS_FILE).expect(&format!(
            "Failed to load settings from file {}",
            SETTINGS_FILE
        )))
    };
}

pub fn _update_settings(new_settings: Settings) -> Result<()> {
    write_settings(SETTINGS_FILE, &new_settings)?;
    *SETTINGS.write().unwrap() = new_settings;
    Ok(())
}

fn load_settings(path: impl AsRef<Path>) -> Result<Settings> {
    info!("Reading settings...");
    let path = path.as_ref();
    let settings = if path.is_file() {
        let mut settings_file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .context(format!("Failed to open settings file {}", path.display()))?;
        let mut buf = String::new();
        settings_file
            .read_to_string(&mut buf)
            .context(format!("Failed to read settings file {}", path.display()))?;
        ron::de::from_str(&buf)
            .context(format!("Failed to parse settings file {}", path.display()))?
    } else {
        std::fs::create_dir_all(&SETTINGS_PATH)?;
        Settings::default()
    };

    write_settings(path, &settings)?;

    Ok(settings)
}

fn write_settings(path: impl AsRef<Path>, settings: &Settings) -> Result<()> {
    info!("Writing settings...");
    let path = path.as_ref();
    let mut settings_file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(&path)
        .context(format!("Failed to open settings file {}", path.display()))?;
    let string = ron::ser::to_string(settings).context("Failed to serialize settings")?;
    settings_file
        .write(string.as_bytes())
        .context(format!("Failed to write settings file {}", path.display()))?;

    Ok(())
}

/// Settings of the game
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct Settings {
    pub window_size: (u32, u32),
    pub invert_mouse: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            window_size: (1600, 900),
            invert_mouse: false,
        }
    }
}
