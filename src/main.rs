#[macro_use]
extern crate gfx;

use anyhow::Result;
use log::info;
use std::path::Path;

mod block;
mod data;
mod fps;
mod input;
mod mesh;
mod perlin;
mod physics;
mod registry;
mod settings;
mod singleplayer;
mod texture;
mod ui;
mod window;
mod world;

fn main() -> Result<()> {
    env_logger::init();

    info!("Starting up...");
    let config_folder = Path::new("config");
    let config_file = Path::new("config/Settings.ron");
    let mut settings = settings::load_settings(&config_folder, &config_file)?;
    info!("Current settings: {:?}", settings);
    window::open_window(&mut settings, Box::new(singleplayer::SinglePlayer::new))?;
    Ok(())
}
