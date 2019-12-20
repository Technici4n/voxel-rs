//! Meshing worker, allowing meshing to be performed in a separate thread
use super::meshing::{greedy_meshing, ChunkMeshData};
use crate::render::world::ChunkVertex;
use voxel_rs_common::block::BlockMesh;
use voxel_rs_common::world::chunk::ChunkPos;
use voxel_rs_common::worker::{WorkerState, Worker};

pub type ChunkMesh = (ChunkPos, Vec<ChunkVertex>, Vec<u32>);

pub struct MeshingState {
    block_meshes: Vec<BlockMesh>,
    quads_reuse: Vec<super::meshing::Quad>,
}

impl MeshingState {
    pub fn new(block_meshes: Vec<BlockMesh>) -> Self {
        Self {
            block_meshes,
            quads_reuse: Vec::new(),
        }
    }
}

impl WorkerState<ChunkMeshData, ChunkMesh> for MeshingState {
    fn compute(&mut self, pos: ChunkPos, data: ChunkMeshData) -> ChunkMesh {
        let (vertices, indices, _, _) = greedy_meshing(data, &self.block_meshes, &mut self.quads_reuse);
        (pos, vertices, indices)
    }
}

pub type MeshingWorker = Worker<ChunkMeshData, ChunkMesh, MeshingState>;
