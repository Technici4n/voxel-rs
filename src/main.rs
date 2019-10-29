#[macro_use]
extern crate gfx;

use anyhow::Result;
use log::info;

mod input;
mod perlin;
mod settings;
mod ui;
mod window;
mod world;
mod mesh;

fn main() -> Result<()> {
    env_logger::init();

    info!("Starting up...");
    info!("Current settings: {:?}", *settings::SETTINGS);
    let mut window = window::Window::new()?;
    while window.running {
        window.process_events()?;
        window.tick()?;
        window.render()?;
    }
    Ok(())
}
