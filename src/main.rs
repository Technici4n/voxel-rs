use anyhow::Result;
use log::info;

mod window;

fn main() -> Result<()> {
    env_logger::init();

    info!("Starting up...");
    let mut window = window::Window::new()?;
    while window.running {
        window.process_events();
        window.render()?;
    }
    Ok(())
}
