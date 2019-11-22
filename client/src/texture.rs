use anyhow::Result;
use gfx::handle::ShaderResourceView;
use image::{ImageBuffer, Rgba};

/// Load an image into a texture
pub fn load_image<F, R>(
    factory: &mut F,
    image: ImageBuffer<Rgba<u8>, Vec<u8>>,
) -> Result<ShaderResourceView<R, [f32; 4]>>
where
    F: gfx::Factory<R>,
    R: gfx::Resources,
{
    // Only squared images are allowed
    // TODO: check for power of two
    assert_eq!(image.width(), image.height());
    let image_size = image.width();
    dbg!(image_size);
    // Generate mipmaps
    let mut mipmaps = Vec::new();
    mipmaps.push(Vec::from(&*image));
    for level in 1..5 {
        // 5 mip maps only
        let current_size = (image_size >> level) as usize;
        if current_size == 0 {
            break;
        }
        let previous_size = (image_size >> (level - 1)) as usize;
        let mut new_layer = Vec::with_capacity(current_size * current_size * 4);
        let previous_layer = mipmaps.last().unwrap();
        for row in 0..current_size {
            for col in 0..current_size {
                for color in 0..4 {
                    new_layer.push(
                        ((previous_layer[2 * row * previous_size * 4 + 2 * col * 4 + color] as u16
                            + previous_layer
                                [2 * row * previous_size * 4 + (2 * col + 1) * 4 + color]
                                as u16
                            + previous_layer
                                [(2 * row + 1) * previous_size * 4 + 2 * col * 4 + color]
                                as u16
                            + previous_layer
                                [(2 * row + 1) * previous_size * 4 + (2 * col + 1) * 4 + color]
                                as u16)
                            / 4) as u8,
                    );
                }
            }
        }
        mipmaps.push(new_layer);
    }
    // Send texture to GPU
    let texture_kind = gfx::texture::Kind::D2(
        image_size as u16,
        image_size as u16,
        gfx::texture::AaMode::Single,
    );
    let (_, texture_view) = factory.create_texture_immutable_u8::<gfx::format::Rgba8>(
        texture_kind,
        gfx::texture::Mipmap::Provided,
        &[
            &mipmaps[0],
            &mipmaps[1],
            &mipmaps[2],
            &mipmaps[3],
            &mipmaps[4],
        ],
    )?;
    Ok(texture_view)
}
