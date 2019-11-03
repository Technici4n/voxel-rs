use anyhow::Result;
use gfx::handle::ShaderResourceView;
use std::path::PathBuf;
use texture_packer::{TexturePacker, TexturePackerConfig};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TextureRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

const MAX_TEXTURE_SIZE: u32 = 2048;

const TEXTURE_PACKER_CONFIG: TexturePackerConfig = TexturePackerConfig {
    max_width: MAX_TEXTURE_SIZE,
    max_height: MAX_TEXTURE_SIZE,
    allow_rotation: false,
    border_padding: 0,
    texture_padding: 0,
    trim: false,
    texture_outlines: false,
};

/// Load given textures to a unique texture atlas
pub fn load_textures<F, R>(
    factory: &mut F,
    textures: Vec<PathBuf>,
) -> Result<(ShaderResourceView<R, [f32; 4]>, Vec<TextureRect>)>
where
    F: gfx::Factory<R>,
    R: gfx::Resources,
{
    use image::{GenericImage, ImageBuffer};
    use texture_packer::{exporter::ImageExporter, importer::ImageImporter};

    let mut packer = TexturePacker::new_skyline(TEXTURE_PACKER_CONFIG);
    for (i, path) in textures.iter().enumerate() {
        packer.pack_own(
            format!("{}", i),
            ImageImporter::import_from_file(path).expect("Failed to read texture to pack"),
        );
    }

    let mut texture_buffer = ImageBuffer::new(MAX_TEXTURE_SIZE, MAX_TEXTURE_SIZE);
    texture_buffer.copy_from(
        &ImageExporter::export(&packer).expect("Failed to export texture from packer"),
        0,
        0,
    );
    let texture_kind = gfx::texture::Kind::D2(
        MAX_TEXTURE_SIZE as u16,
        MAX_TEXTURE_SIZE as u16,
        gfx::texture::AaMode::Single,
    );
    let (_, texture_view) = factory.create_texture_immutable_u8::<gfx::format::Rgba8>(
        texture_kind,
        gfx::texture::Mipmap::Provided,
        &[&texture_buffer],
    )?;
    Ok((
        texture_view,
        (0..textures.len())
            .map(|i| {
                let frame = packer
                    .get_frame(&format!("{}", i))
                    .expect("Texture packer frame key doesn't exist")
                    .frame;
                TextureRect {
                    x: frame.x as f32 / MAX_TEXTURE_SIZE as f32,
                    y: frame.y as f32 / MAX_TEXTURE_SIZE as f32,
                    width: frame.w as f32 / MAX_TEXTURE_SIZE as f32,
                    height: frame.h as f32 / MAX_TEXTURE_SIZE as f32,
                }
            })
            .collect(),
    ))
}
