use serde::Deserialize;

pub mod mesh;

#[derive(Deserialize)]
pub enum BlockType {
    Air,
    NormalCube,
}

#[derive(Deserialize)]
pub enum BlockMeshData {
    NoMesh,
    NormalCube(Vec<String>),
}

#[derive(Deserialize)]
pub struct BlockData {
    #[serde(rename = "type")]
    pub block_type: BlockType,
    pub mesh_data: BlockMeshData,
}

pub struct Block {
    pub name: String,
    pub block_type: BlockType,
}
