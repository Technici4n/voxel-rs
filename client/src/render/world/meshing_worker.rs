//! Meshing worker, allowing meshing to be performed in a separate thread
use super::meshing::{greedy_meshing, ChunkMeshData};
use crate::render::world::ChunkVertex;
use crossbeam_channel::{Receiver, Sender, TrySendError, bounded, unbounded};
use voxel_rs_common::block::BlockMesh;
use voxel_rs_common::world::chunk::ChunkPos;
use voxel_rs_common::worker::{WorkerState, Worker};

pub type ChunkMesh = (ChunkPos, Vec<ChunkVertex>, Vec<u32>);

pub struct MeshingWorker {
    to_worker: Sender<ChunkMeshData>,
    from_worker: Receiver<ChunkMesh>,
}

static WORKER_CHANNEL_SIZE: usize = 20; // TODO: better size?

impl MeshingWorker {
    pub fn new(block_meshes: Vec<BlockMesh>) -> Self {
        let (in_sender, in_receiver) = bounded::<ChunkMeshData>(WORKER_CHANNEL_SIZE);
        let (out_sender, out_receiver) = bounded::<ChunkMesh>(WORKER_CHANNEL_SIZE);

        std::thread::spawn(move || { // TODO: debug timing
            let block_meshes = block_meshes;
            let mut quads_reuse = Vec::new();
            while let Ok(data) = in_receiver.recv() {
                let pos = (*data.chunk).pos;
                let (vertices, indices, _, _) = greedy_meshing(data, &block_meshes, &mut quads_reuse);
                match out_sender.send((pos, vertices, indices)) {
                    Ok(()) => (),
                    Err(_) => break,
                }
            }
        });

        Self {
            to_worker: in_sender,
            from_worker: out_receiver,
        }
    }

    pub fn enqueue(&self, data: ChunkMeshData) -> Result<(), ChunkMeshData> {
        self.to_worker.try_send(data).map_err(|e| match e {
            TrySendError::Full(data) => data,
            TrySendError::Disconnected(_) => unreachable!("Meshing worker channel disconnected"),
        })
    }

    pub fn get_mesh(&self) -> Option<ChunkMesh> {
       self.from_worker.try_recv().ok()
    }
}
