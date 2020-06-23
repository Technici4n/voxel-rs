//! Generic worker, allowing a computation to be performed in a separate thread
use std::{
    marker::PhantomData,
    time::Instant,
};
use crossbeam_channel::{Receiver, Sender, TrySendError, bounded};
use crate::{debug::send_worker_perf, time::AverageTimeCounter};

/// A type that takes inputs of type `Input` produces outputs of type `Output`.
pub trait WorkerState<Input, Output> {
    fn compute(&mut self, input: Input) -> Output;
}

/// A generic worker allowing to offload expensive computations to other threads.
/// The worker will try to process the inputs in order.
/// `Input`: the input type
/// `Output`: the output type
/// `State`: the worker state
pub struct Worker<Input: Send + 'static, Output: Send + 'static, State: WorkerState<Input, Output> + Send + 'static> {
    to_worker: Sender<Input>,
    from_worker: Receiver<Output>,
    _phantom: PhantomData<State>,
}

impl<Input: Send + 'static, Output: Send + 'static, State: WorkerState<Input, Output> + Send + 'static> Worker<Input, Output, State> {
    /// Start a new worker with the given state using the provided channel size. The name is used for debug printing.
    pub fn new(state: State, channel_size: usize, name: String) -> Self {
        let (in_sender, in_receiver) = bounded::<Input>(channel_size);
        let (out_sender, out_receiver) = bounded::<Output>(channel_size);

        std::thread::spawn(move || { // TODO: debug timing
            let mut state = state;
            let mut timing = AverageTimeCounter::new();
            while let Ok(input) = in_receiver.recv() {
                // Compute
                let t1 = Instant::now();
                let output = state.compute(input);
                let t2 = Instant::now();
                timing.add_time(t2 - t1);

                // Send debug info
                send_worker_perf("Workers", &name, &name, timing.average_time_micros() as f32, timing.average_iter_per_sec(), 0);

                // Send result
                match out_sender.send(output) {
                    Ok(()) => (),
                    Err(_) => break,
                }
            }
        });

        Self {
            to_worker: in_sender,
            from_worker: out_receiver,
            _phantom: PhantomData,
        }
    }

    /// Try to enqueue a new input in the worker queue. Doesn't block. Will return the input if the queue is full.
    pub fn enqueue(&self, input: Input) -> Result<(), Input> {
        self.to_worker.try_send(input).map_err(|e| match e {
            TrySendError::Full(input) => input,
            TrySendError::Disconnected(_) => unreachable!("Worker channel disconnected"),
        })
    }

    /// Try to get a new output from the worker. Doesn't block. Will return None if there is no available output.
    pub fn get_result(&self) -> Option<Output> {
       self.from_worker.try_recv().ok()
    }
}