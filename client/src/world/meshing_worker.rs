use crate::world::meshing::{greedy_meshing, AdjChunkOccl};
use crate::world::renderer::Vertex;
use std::collections::{HashMap, VecDeque};
use std::sync::mpsc::{channel, Receiver, Sender};
use voxel_rs_common::block::BlockMesh;
use voxel_rs_common::world::chunk::{Chunk, ChunkPos};

pub type ChunkMesh = (ChunkPos, Vec<Vertex>, Vec<u32>);

/// A worker that runs the meshing on one or more other threads.
pub struct MeshingWorker {
    sender: Sender<ToOtherThread>,
    receiver: Receiver<ChunkMesh>,
}

/// Message sent to the other threads.
enum ToOtherThread {
    Enqueue(Chunk, AdjChunkOccl),
    Dequeue(ChunkPos),
}

impl MeshingWorker {
    /// Create a new `MeshingWorker`, using the given block meshes.
    pub fn new(block_meshes: Vec<BlockMesh>) -> Self {
        let (sender1, receiver1) = channel();
        let (sender2, receiver2) = channel();

        std::thread::spawn(move || {
            launch_worker(sender2, receiver1, block_meshes);
        });

        Self {
            sender: sender1,
            receiver: receiver2,
        }
    }

    /// Enqueue a chunk
    pub fn enqueue_chunk(&mut self, chunk: Chunk, adj: AdjChunkOccl) {
        self.sender
            .send(ToOtherThread::Enqueue(chunk, adj))
            .unwrap();
    }

    /// Dequeue a chunk from processing if it's still in the queue.
    /// The chunk may still be meshed, but the worker will try to avoid it.
    pub fn dequeue_chunk(&mut self, pos: ChunkPos) {
        self.sender.send(ToOtherThread::Dequeue(pos)).unwrap();
    }

    /// Get the processed chunks
    pub fn get_processed_chunks(&mut self) -> Vec<ChunkMesh> {
        let mut processed_chunks = Vec::new();
        while let Ok(chunk) = self.receiver.try_recv() {
            processed_chunks.push(chunk);
        }
        processed_chunks
    }
}

/// Start the worker using the given channels.
/// The worker will `panic!` once the sending channel gets disconnected.
fn launch_worker(
    sender: Sender<ChunkMesh>,
    receiver: Receiver<ToOtherThread>,
    block_meshes: Vec<BlockMesh>,
) {
    let mut quads = Vec::new();

    let mut queued_chunks = HashMap::new();
    let mut ordered_positions = VecDeque::new();
    loop {
        // Process all messages
        while let Some(message) = {
            if queued_chunks.len() > 0 {
                // Either there are pending chunks and we want to process them, so we don't block
                receiver.try_recv().ok()
            } else {
                // Or there are no pending chunks, and we block to save CPU
                receiver.recv().ok()
            }
        } {
            match message {
                ToOtherThread::Enqueue(chunk, adj) => {
                    ordered_positions.push_back(chunk.pos);
                    queued_chunks.insert(chunk.pos, (chunk, adj));
                }
                ToOtherThread::Dequeue(pos) => {
                    queued_chunks.remove(&pos);
                }
            }
        }

        // Mesh the first chunk that is in the queue
        while let Some(chunk_pos) = ordered_positions.pop_front() {
            if let Some((chunk, adj)) = queued_chunks.remove(&chunk_pos) {
                let (vertices, indices, _, _) =
                    greedy_meshing(&chunk, Some(adj), &block_meshes, &mut quads);
                sender.send((chunk_pos, vertices, indices)).unwrap();
                break;
            }
        }
    }
}
