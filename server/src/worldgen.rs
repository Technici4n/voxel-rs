use std::collections::HashSet;
use std::sync::mpsc::{channel, Receiver, Sender};
use voxel_rs_common::block::Block;
use voxel_rs_common::registry::Registry;
use voxel_rs_common::world::chunk::{Chunk, ChunkPos};
use voxel_rs_common::world::WorldGenerator;

/// A worker that runs the world generation on one or more other threads.
pub struct WorldGenerationWorker {
    sender: Sender<ToOtherThread>,
    receiver: Receiver<Chunk>,
}

/// Message sent to the other threads.
enum ToOtherThread {
    Enqueue(ChunkPos),
    Dequeue(ChunkPos),
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
                }
                ToOtherThread::Dequeue(pos) => {
                    queued_chunks.remove(&pos);
                }
            }
        }

        // Mesh the first chunk
        if let Some(&chunk_pos) = queued_chunks.iter().next() {
            queued_chunks.remove(&chunk_pos);
            let chunk = world_generator.generate_chunk(chunk_pos, &block_registry);
            sender.send(chunk).unwrap();
        }
    }
}
