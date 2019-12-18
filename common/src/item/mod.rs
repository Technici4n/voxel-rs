use serde::Deserialize;

pub type ItemId = u32;

/// The type of an item. It contains the behavior and the texture of the item.
/// This is the data provided by the creator of the item.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename = "Item")]
pub enum ItemType {
    NormalItem { texture: String },
}

/// The mesh of an item
#[derive(Debug, Clone)]
pub enum ItemMesh {
    /// Simply a mesh
    SimpleMesh {
        /// Id of the mesh
        mesh_id: u32,
        /// Scale of the mesh
        scale: f32,
        /// Center of the mesh, relative to the cube at position (0, 0, 0), before scaling
        mesh_center: (f32, f32, f32),
    },
}

/// A general item in-memory representation
#[derive(Debug, Clone)]
pub struct Item {
    pub name: String,
    pub ty: ItemType,
}
