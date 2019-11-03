use crate::{
    block::{mesh::BlockMesh, Block, BlockData, BlockMeshData, BlockType},
    registry::Registry,
};
use anyhow::{Context, Result};
use log::info;
use std::fs;
use std::io::Read;
use std::path::PathBuf;

pub struct Data {
    pub blocks: Registry<Block>,
    pub meshes: Vec<BlockMesh>,
    pub texture_atlas: gfx::handle::ShaderResourceView<gfx_device_gl::Resources, [f32; 4]>,
}

pub fn load_data(factory: &mut gfx_device_gl::Factory, data_directory: PathBuf) -> Result<Data> {
    info!("Loading data from directory {}", data_directory.display());

    // Load textures
    let mut textures: Vec<PathBuf> = Vec::new();
    let mut texture_registry: Registry<()> = Default::default();
    let textures_directory = data_directory.join("textures");
    info!(
        "Loading textures from directory {}",
        textures_directory.display()
    );
    for dir_entry in fs::read_dir(textures_directory).context("couldn't read textures directory")? {
        let dir_entry = dir_entry.context("failed to read directory entry")?;
        if dir_entry
            .file_type()
            .context("failed to get file type")?
            .is_file()
        {
            let file_path = dir_entry.path();

            texture_registry.register(
                file_path
                    .file_stem()
                    .context("failed to get file stem")?
                    .to_str()
                    .unwrap()
                    .to_owned(),
                (),
            )?;
            textures.push(file_path);
        }
    }

    let (texture_atlas, texture_rects) = crate::texture::load_textures(factory, textures)?;

    // Load blocks
    let mut block_datas: Vec<(String, BlockData)> = Vec::new();
    let blocks_directory = data_directory.join("blocks");
    info!(
        "Loading blocks from directory {}",
        blocks_directory.display()
    );
    for dir_entry in fs::read_dir(blocks_directory).context("couldn't read block directory")? {
        let dir_entry = dir_entry.context("failed to read directory entry")?;
        if dir_entry
            .file_type()
            .context("failed to get file type")?
            .is_file()
        {
            let file_path = dir_entry.path();

            match file_path.extension() {
                None => panic!("No file extension"),
                Some(ext) => {
                    if ext == "ron" {
                        let mut file = fs::File::open(file_path.clone())
                            .context("couldn't open .ron block file")?;
                        let mut buffer = String::new();
                        file.read_to_string(&mut buffer)?;
                        block_datas.push((
                            file_path
                                .file_stem()
                                .context("failed to get file stem")?
                                .to_str()
                                .unwrap()
                                .to_owned(),
                            ron::de::from_str(&buffer)
                                .context("failed to parse .ron block file")?,
                        ));
                    } else {
                        panic!("Unsupported file extension");
                    }
                }
            }
        }
    }

    info!("Processing collected block and texture data");
    let mut blocks = Registry::default();
    let mut meshes = Vec::new();
    // Add air
    blocks.register(
        "air".to_owned(),
        Block {
            name: "air".to_owned(),
            block_type: BlockType::Air,
        },
    )?;
    meshes.push(BlockMesh::Empty);

    for (name, block_data) in block_datas.into_iter() {
        let block = Block {
            name: name.clone(),
            block_type: block_data.block_type,
        };
        blocks.register(name, block)?;
        let mesh = match block_data.mesh_data {
            BlockMeshData::NoMesh => BlockMesh::Empty,
            BlockMeshData::NormalCube(names) => BlockMesh::FullCube {
                textures: [
                    texture_rects[texture_registry.get_id_by_name(&names[0]).unwrap() as usize],
                    texture_rects[texture_registry.get_id_by_name(&names[1]).unwrap() as usize],
                    texture_rects[texture_registry.get_id_by_name(&names[2]).unwrap() as usize],
                    texture_rects[texture_registry.get_id_by_name(&names[3]).unwrap() as usize],
                    texture_rects[texture_registry.get_id_by_name(&names[4]).unwrap() as usize],
                    texture_rects[texture_registry.get_id_by_name(&names[5]).unwrap() as usize],
                ],
            },
        };
        meshes.push(mesh);
    }

    info!("Data successfully loaded");
    Ok(Data {
        blocks,
        meshes,
        texture_atlas,
    })
}
