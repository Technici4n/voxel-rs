use log::info;
use std::path::Path;

/// Load a shader from a file.
pub fn load_shader<P: AsRef<Path>>(path: P) -> String {
    info!("Loading shader from {}", path.as_ref().display());
    std::fs::read_to_string(path).expect("Couldn't read shader from file")
}