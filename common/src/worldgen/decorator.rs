use crate::world::BlockPos;
use std::collections::HashSet;

// TODO : Create a procedural decorator
/// Struct used to generate pre-defined groups of block in the world
pub(crate) struct Decorator {
    pub number_of_try: u32, // number of times this will be try to be spawn/chunks
    pub block_start_whitelist: HashSet<u16>, // the blocks allowed to be the start of the Decorator
    pub pass: Vec<DecoratorPass>, // the pass of each block for the decorator
}

pub struct DecoratorPass {
    pub block_type: u16,                  // the block type
    pub block_non_blocking: HashSet<u16>, // list of the block that will no be replaced but will not block the strucutre to spawn
    pub block_whitelist: HashSet<u16>,    // the blocks this block can replace
    pub block_pos: Vec<BlockPos>, // the relative position of the block relative to the structure center
}

impl DecoratorPass {
    pub fn new(block_type: u16) -> Self {
        let mut block_whitelist = HashSet::new();
        block_whitelist.insert(0);
        block_whitelist.insert(block_type);
        DecoratorPass {
            block_type,
            block_non_blocking: HashSet::new(),
            block_whitelist,
            block_pos: Vec::new(),
        }
    }
}
/// Useful macro to create set
#[macro_export]
macro_rules! set {
    ( $( $x:expr ),* ) => {  // Match zero or more comma delimited items
        {
            let mut temp_set = HashSet::new();  // Create a mutable HashSet
            $(
                temp_set.insert($x); // Insert each item matched into the HashSet
            )*
            temp_set // Return the populated HashSet
        }
    };
}
