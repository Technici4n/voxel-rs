//! Meshing worker, allowing meshing to be performed in a separate thread
use super::meshing::{greedy_meshing, ChunkMeshData};
use crate::render::world::ChunkVertex;
use voxel_rs_common::block::BlockMesh;
use voxel_rs_common::world::ChunkPos;
use voxel_rs_common::worker::{WorkerState, Worker};

pub type ChunkMesh = (ChunkPos, Vec<ChunkVertex>, Vec<u32>);
pub type MeshingWorker = Worker<ChunkMeshData, ChunkMesh, MeshingState>;

pub fn start_meshing_worker(block_meshes: Vec<BlockMesh>) -> MeshingWorker {
    MeshingWorker::new(
        MeshingState::new(block_meshes),
        WORKER_CHANNEL_SIZE,
        "Meshing".to_owned(),
    )
}

pub struct MeshingState {
    block_meshes: Vec<BlockMesh>,
    quads_reuse: Vec<super::meshing::Quad>,
}

impl MeshingState {
    pub(self) fn new(block_meshes: Vec<BlockMesh>) -> Self {
        Self {
            block_meshes,
            quads_reuse: Vec::new(),
        }
    }
}

impl WorkerState<ChunkMeshData, ChunkMesh> for MeshingState {
    fn compute(&mut self, input: ChunkMeshData) -> ChunkMesh {
        let pos = input.chunk.pos;
        let (vertices, indices, _, _) = greedy_meshing(input, &self.block_meshes, &mut self.quads_reuse);
        (pos, vertices, indices)
    }
}

static WORKER_CHANNEL_SIZE: usize = 20; // TODO: better size?
