use crate::data::TextureRect;
use serde::Deserialize;

pub type BlockId = u16;

/// The type of a block. It contains the behavior and the mesh of the block.
/// This is the data provided by the creator of the block.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename = "Block")]
pub enum BlockType {
    Air, // TODO: skip when deserializing
    NormalCube { face_textures: Vec<String> },
}

/// A general block in-memory representation.
#[derive(Debug, Clone)]
pub struct Block {
    pub name: String,
    pub block_type: BlockType,
}

/// The mesh of a block.
#[derive(Debug, Clone)]
pub enum BlockMesh {
    /// No mesh
    Empty,
    /// A usual full cube
    FullCube { textures: [TextureRect; 6] },
}

impl BlockMesh {
    pub fn is_opaque(&self) -> bool {
        match self {
            Self::Empty => false,
            Self::FullCube { .. } => true,
        }
    }
}
