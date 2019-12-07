use serde::Deserialize;

pub type ItemId = u32;

/// The type of an item. It contains the behavior and the texture of the item.
/// This is the data provided by the creator of the item.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename = "Block")]
pub enum ItemType {
    NormalItem { texture: String },
}