//! Meshing worker, allowing meshing to be performed in a separate thread
use super::meshing::{greedy_meshing, ChunkMeshData};
use crate::render::world::ChunkVertex;
use std::collections::{BTreeMap, HashMap};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Instant;
use voxel_rs_common::block::BlockMesh;
use voxel_rs_common::debug::send_debug_info;
use voxel_rs_common::time::AverageTimeCounter;
use voxel_rs_common::world::chunk::ChunkPos;

pub type ChunkMesh = (ChunkPos, Vec<ChunkVertex>, Vec<u32>);

/// A worker that runs the meshing on one or more other threads.
pub struct MeshingWorker {
    sender: Sender<ToOtherThread>,
    receiver: Receiver<ChunkMesh>,
}

/// Message sent to the other threads.
enum ToOtherThread {
    Enqueue(ChunkMeshData),
    SetPriority(ChunkPos, u64),
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
    pub fn enqueue_chunk(&mut self, data: ChunkMeshData) {
        self.sender.send(ToOtherThread::Enqueue(data)).unwrap();
    }

    /// Update a chunk's priority
    pub fn update_chunk_priority(&mut self, pos: ChunkPos, priority: u64) {
        self.sender
            .send(ToOtherThread::SetPriority(pos, priority))
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
    let mut priorities = BTreeMap::new();
    let mut meshing_timing = AverageTimeCounter::new();
    loop {
        send_debug_info(
            "Chunks",
            "meshing",
            format!("Meshing pending chunks = {}", queued_chunks.len()),
        );
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
                ToOtherThread::Enqueue(data) => {
                    let pos = data.chunk.pos;
                    queued_chunks.insert(pos, data);
                    priorities
                        .entry(u64::max_value())
                        .or_insert_with(Vec::new)
                        .push(pos);
                }
                ToOtherThread::SetPriority(pos, priority) => {
                    if queued_chunks.contains_key(&pos) {
                        priorities
                            .entry(priority)
                            .or_insert_with(Vec::new)
                            .push(pos);
                    }
                }
                ToOtherThread::Dequeue(pos) => {
                    queued_chunks.remove(&pos);
                }
            }
        }

        // Mesh the chunk with the lowest priority
        'outer: while let Some((&priority, positions)) = priorities.iter_mut().next() {
            while let Some(chunk_pos) = positions.pop() {
                if let Some(data) = queued_chunks.remove(&chunk_pos) {
                    let t1 = Instant::now();
                    let (vertices, indices, _, _) = greedy_meshing(data, &block_meshes, &mut quads);
                    let t2 = Instant::now();
                    meshing_timing.add_time(t2 - t1);
                    send_debug_info(
                        "Chunks",
                        "averagetime_meshing",
                        format!(
                            "Average time to mesh chunks: {} μs",
                            meshing_timing.average_time_micros()
                        ),
                    );

                    sender.send((chunk_pos, vertices, indices)).unwrap();
                    break 'outer;
                }
            }

            priorities.remove(&priority);
            break;
        }
    }
}
