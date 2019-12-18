use crate::data::vox::VoxelModel;
use crate::data::{TextureRect, MAX_TEXTURE_SIZE};
use image::{ImageBuffer, Rgba};

pub fn generate_item_model(
    texture: TextureRect,
    atlas: &ImageBuffer<Rgba<u8>, Vec<u8>>,
) -> VoxelModel {
    let x = (texture.x * MAX_TEXTURE_SIZE as f32).round() as u32;
    let y = (texture.y * MAX_TEXTURE_SIZE as f32).round() as u32;
    let width = (texture.width * MAX_TEXTURE_SIZE as f32).round() as u32;
    let height = (texture.height * MAX_TEXTURE_SIZE as f32).round() as u32;

    let mut full = Vec::with_capacity((width * height) as usize);
    let mut voxels = Vec::with_capacity((width * height) as usize);

    for u in x..(x + width) {
        for v in (y..(y + height)).rev() {
            let rgba = atlas.get_pixel(u, v);
            if rgba[3] == 255 {
                // Not transparent
                full.push(true);
                // AGBR
                voxels.push(((rgba[2] as u32) << 16) + ((rgba[1] as u32) << 8) + rgba[0] as u32);
            } else {
                full.push(false);
                voxels.push(0);
            }
        }
    }

    VoxelModel {
        size_x: width as usize,
        size_y: height as usize,
        size_z: 1,
        voxels,
        full,
    }
}
