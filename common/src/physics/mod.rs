use crate::world::{BlockPos, World};

pub mod aabb;
pub mod camera;
pub mod player;
pub mod simulation;

/// A "block container", i.e. either the client's World or the server's World.
/// This trait allows the physics simulation to work transparently with both World structs.
pub trait BlockContainer {
    fn is_block_full(&self, pos: BlockPos) -> bool;
}

// TODO: remove this when the separate server world is implemented
impl BlockContainer for World {
    fn is_block_full(&self, pos: BlockPos) -> bool {
        // TODO: use the BlockRegistry
        return self.get_block(pos) != 0;
    }
}
