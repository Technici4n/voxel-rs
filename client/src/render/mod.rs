//! Rendering part of the client

/* WebGPU HELPER MODULES */
mod buffers;
mod init;
mod render;
pub use self::buffers::MultiBuffer;
pub use self::render::{clear_color_and_depth, clear_depth, encode_resolve_render_pass, to_u8_slice, buffer_from_slice};

/* OTHER HELPER MODULES */
mod frustum;
pub use self::frustum::Frustum;

/* RENDERING-RESPONSIBLE MODULES */
mod ui;
pub mod world;
pub use self::ui::UiRenderer;
pub use self::world::{Model, WorldRenderer, ChunkVertex};
