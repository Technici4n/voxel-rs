use crate::texture::TextureRect;

pub enum BlockMesh {
    Empty,
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
