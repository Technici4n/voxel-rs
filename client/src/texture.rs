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
    let texture_kind = gfx::texture::Kind::D2(
        image.width() as u16,
        image.height() as u16,
        gfx::texture::AaMode::Single,
    );
    let (_, texture_view) = factory.create_texture_immutable_u8::<gfx::format::Rgba8>(
        texture_kind,
        gfx::texture::Mipmap::Provided,
        &[&*image],
    )?;
    Ok(texture_view)
}
