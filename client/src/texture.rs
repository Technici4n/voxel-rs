use wgpu::util::{BufferInitDescriptor, DeviceExt};
use image::{ImageBuffer, Rgba};
use log::info;

const MIPMAP_LEVELS: u32 = 5;

/// Load an image into a texture
pub fn load_image(
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    image: ImageBuffer<Rgba<u8>, Vec<u8>>,
) -> wgpu::Texture {
    info!("Loading image...");
    // Only squared images are allowed
    // TODO: check for power of two
    assert_eq!(image.width(), image.height());
    let image_size = image.width();
    // Generate mipmaps
    let mut mipmaps = Vec::new();
    mipmaps.push(Vec::from(&*image));
    for level in 1..MIPMAP_LEVELS {
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
    // Create texture
    info!("Creating texture");
    let texture_descriptor = wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: image_size,
            height: image_size,
            depth: 1,
        },
        mip_level_count: MIPMAP_LEVELS,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsage::COPY_DST | wgpu::TextureUsage::SAMPLED,
    };
    let texture = device.create_texture(&texture_descriptor);
    // Send texture to GPU

    for level in 0..MIPMAP_LEVELS {
        info!("Copying mipmap level {mipmap_level}", mipmap_level = level);
        let current_size = image_size >> level;
        let src_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            usage: wgpu::BufferUsage::COPY_SRC,
            contents: &mipmaps[level as usize]
        });
        let buffer_view = wgpu::BufferCopyView {
            layout: wgpu::TextureDataLayout {
                offset: 0,
                rows_per_image: current_size,
                bytes_per_row: 4 * current_size,
            },
            buffer: &src_buffer,
        };
        let texture_view = wgpu::TextureCopyView {
            texture: &texture,
            mip_level: level,
            origin: wgpu::Origin3d {
                x: 0,
                y: 0,
                z: 0,
            },
        };
        encoder.copy_buffer_to_texture(
            buffer_view,
            texture_view,
            wgpu::Extent3d {
                width: current_size,
                height: current_size,
                depth: 1,
            },
        );
    }
    info!("Texture loading successful");
    texture
}
