use anyhow::Result;
use log::{error, info};
use std::path::Path;
use voxel_rs_common::network::dummy;
use voxel_rs_server::launch_server;

mod fps;
mod gui;
mod input;
//mod mainmenu; TODO: fix this
mod render;
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
    let config_file = Path::new("config/settings.toml");
    let settings = settings::load_settings(&config_folder, &config_file)?;
    info!("Current settings: {:?}", settings);

    let (client, server) = dummy::new();

    std::thread::spawn(move || {
        if let Err(e) = launch_server(Box::new(server)) {
            // TODO: rewrite this error reporting
            error!(
                "Error happened in the server code: {}\nPrinting chain:\n{}",
                e,
                e.chain()
                    .enumerate()
                    .map(|(i, e)| format!("{}: {}", i, e))
                    .collect::<Vec<_>>()
                    .join("\n")
            );
        }
    });

    window::open_window(
        settings,
        Box::new(singleplayer::SinglePlayer::new_factory(Box::new(client))),
    )
}
