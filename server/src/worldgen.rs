use std::collections::{BTreeMap, HashSet};
use std::sync::mpsc::{channel, Receiver, Sender};
use voxel_rs_common::{
    block::Block,
    registry::Registry,
    world::chunk::{Chunk, ChunkPos},
    world::WorldGenerator,
};

/// A worker that runs the world generation on one or more other threads.
/// Chunks are processed lowest priority first.
pub struct WorldGenerationWorker {
    sender: Sender<ToOtherThread>,
    receiver: Receiver<Chunk>,
}

/// Message sent to the other threads.
enum ToOtherThread {
    Enqueue(ChunkPos),
    Dequeue(ChunkPos),
    SetPriority(ChunkPos, u64),
}

impl WorldGenerationWorker {
    /// Create a new `WorldGenerationWorker`, using the given `WorldGenerator`.
    pub fn new(
        world_generator: Box<dyn WorldGenerator + Send>,
        block_registry: Registry<Block>,
    ) -> Self {
        let (sender1, receiver1) = channel();
        let (sender2, receiver2) = channel();

        std::thread::spawn(move || {
            launch_worker(sender2, receiver1, world_generator, block_registry);
        });

        Self {
            sender: sender1,
            receiver: receiver2,
        }
    }

    /// Enqueue a chunk
    pub fn enqueue_chunk(&mut self, pos: ChunkPos) {
        self.sender.send(ToOtherThread::Enqueue(pos)).unwrap();
    }

    /// Dequeue a chunk from processing if it's still in the queue.
    /// The chunk may still be generated, but the worker will try to avoid it.
    pub fn dequeue_chunk(&mut self, pos: ChunkPos) {
        self.sender.send(ToOtherThread::Dequeue(pos)).unwrap();
    }

    /// Set the priority of a chunk.
    /// Has no effect if the chunk is not queued
    pub fn set_chunk_priority(&mut self, pos: ChunkPos, priority: u64) {
        self.sender
            .send(ToOtherThread::SetPriority(pos, priority))
            .unwrap();
    }

    /// Get the processed chunks
    pub fn get_processed_chunks(&mut self) -> Vec<Chunk> {
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
    sender: Sender<Chunk>,
    receiver: Receiver<ToOtherThread>,
    mut world_generator: Box<dyn WorldGenerator>,
    block_registry: Registry<Block>,
) {
    let mut queued_chunks = HashSet::new();
    let mut priorities = BTreeMap::new();
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
                ToOtherThread::Enqueue(pos) => {
                    queued_chunks.insert(pos);
                    priorities
                        .entry(u64::max_value())
                        .or_insert_with(Vec::new)
                        .push(pos);
                }
                ToOtherThread::Dequeue(pos) => {
                    queued_chunks.remove(&pos);
                }
                ToOtherThread::SetPriority(pos, priority) => {
                    priorities
                        .entry(priority)
                        .or_insert_with(Vec::new)
                        .push(pos);
                }
            }
        }

        // Find chunk with the lowest priority
        'outer: while let Some((&priority, positions)) = priorities.iter_mut().next() {
            while let Some(pos) = positions.pop() {
                if queued_chunks.remove(&pos) {
                    // Generate the chunk it if it is still queued
                    let chunk = world_generator.generate_chunk(pos, &block_registry);
                    sender.send(chunk).unwrap();
                    break 'outer;
                }
            }

            priorities.remove(&priority);
            break;
        }
    }
}
