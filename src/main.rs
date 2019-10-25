use anyhow::Result;
use log::info;

mod chunk;
mod meshing;
mod perlin;
mod settings;
mod ui;
mod window;

fn main() -> Result<()> {
    env_logger::init();

    info!("Starting up...");
    info!("Current settings: {:?}", *settings::SETTINGS);
    let mut window = window::Window::new()?;
    while window.running {
        window.process_events();
        window.render()?;
    }
    Ok(())
}
