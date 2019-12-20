use crate::world::chunk::ChunkPos;
use std::sync::mpsc::{Sender, Receiver, channel};
use std::marker::PhantomData;
use std::collections::HashMap;

/// A type that takes chunk positions and inputs of type `Input` produces outputs of type `Output`.
pub trait WorkerState<Input, Output> {
    fn compute(&mut self, chunk_pos: ChunkPos, input: Input) -> Output;
}

/// A generic worker allowing to offload expensive chunk computations to other threads.
/// The worker will try to process the chunks that are closest to the players.
/// `Input`: the type
pub struct Worker<Input: Send + 'static, Output: Send + 'static, State: WorkerState<Input, Output> + Send + 'static> {
    sender: Sender<ToOtherThread<Input>>,
    receiver: Receiver<Output>,
    _phantom: PhantomData<State>,
}

/// Message sent to the other thread
enum ToOtherThread<Input: Send> {
    // Enqueue a chunk with some input data.
    Enqueue(ChunkPos, Input),
    // Dequeue a chunk.
    Dequeue(ChunkPos),
    // Update the positions of the players
    SetPositions(Vec<ChunkPos>),
}

impl<Input: Send + 'static, Output: Send + 'static, State: WorkerState<Input, Output> + Send + 'static> Worker<Input, Output, State> {
    /// Create a new `Worker` with the given state.
    pub fn new(state: State) -> Self {
        let (sender1, receiver1) = channel();
        let (sender2, receiver2) = channel();

        std::thread::spawn(move || {
            start_worker_thread(sender2, receiver1, state);
        });

        Self {
            sender: sender1,
            receiver: receiver2,
            _phantom: PhantomData,
        }
    }

    /// Enqueue a chunk for processing
    pub fn enqueue(&self, pos: ChunkPos, input: Input) {
        self.sender.send(ToOtherThread::Enqueue(pos, input)).unwrap();
    }

    /// Dequeue a chunk from processing if it's still in the queue.
    /// The chunk may still be processed but the worker will try to avoid it.
    pub fn dequeue(&self, pos: ChunkPos) {
        self.sender.send(ToOtherThread::Dequeue(pos)).unwrap();
    }

    /// Update the positions of the players
    pub fn update_player_pos(&self, positions: Vec<ChunkPos>) {
        self.sender.send(ToOtherThread::SetPositions(positions)).unwrap();
    }

    /// Get the processed chunks
    pub fn get_processed(&self) -> Vec<Output> {
        let mut processed_chunks = Vec::new();
        while let Ok(output) = self.receiver.try_recv() {
            processed_chunks.push(output);
        }
        processed_chunks
    }
}

/// Start the worker thread using the given channels. The worker will shut down when one of the channels gets disconnected.
fn start_worker_thread<Input: Send, Output: Send, State: WorkerState<Input, Output>>(
    sender: Sender<Output>,
    receiver: Receiver<ToOtherThread<Input>>,
    mut state: State,
) {
    // TODO: add timing
    let mut queued_chunks: HashMap<ChunkPos, Input> = HashMap::new();
    let mut player_positions: Vec<ChunkPos> = Vec::new();
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
                ToOtherThread::Enqueue(pos, input) => {
                    queued_chunks.insert(pos, input);
                }
                ToOtherThread::Dequeue(pos) => {
                    queued_chunks.remove(&pos);
                }
                ToOtherThread::SetPositions(positions) => {
                    player_positions = positions;
                }
            }
        }

        // Sort the queued chunks by distance to closest player
        let mut queued_chunks_vec = queued_chunks.keys().cloned().collect::<Vec<_>>();
        // TODO: cache key if necessary
        queued_chunks_vec.sort_unstable_by_key(|pos| {
            let mut min_distance = 1_000_000_000;
            for player_pos in &player_positions {
                min_distance = u64::min(
                    min_distance,
                    player_pos.squared_euclidian_distance(*pos),
                );
            };
            min_distance
        });

        // TODO: process multiple chunks one after the other if necessary
        if let Some(&next_chunk) = queued_chunks_vec.iter().next() {
            let input = queued_chunks.remove(&next_chunk).unwrap();
            let output = state.compute(next_chunk, input);
            if let Err(_) = sender.send(output) {
                // The sender disconnected
                return;
            }
        }
    }
}