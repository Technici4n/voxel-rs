use rand::{
    thread_rng,
    distributions::{Distribution, Uniform},
};
use std::collections::{BinaryHeap, HashMap};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use voxel_rs_network::Socket;
use super::SocketAddr;

lazy_static::lazy_static! {
    static ref PACKET_QUEUES: Mutex<HashMap<SocketAddr, BinaryHeap<SentPacket>>> = Mutex::new(HashMap::new());
}

#[derive(Eq)]
struct SentPacket {
    arrival_time: Instant,
    sender: SocketAddr,
    data: Vec<u8>,
}

impl Ord for SentPacket {
    fn cmp(&self, other: &SentPacket) -> std::cmp::Ordering {
        other.arrival_time.cmp(&self.arrival_time)
    }
}

impl PartialOrd for SentPacket {
    fn partial_cmp(&self, other: &SentPacket) -> Option<std::cmp::Ordering> {
        other.arrival_time.partial_cmp(&self.arrival_time)
    }
}

impl PartialEq for SentPacket {
    fn eq(&self, other: &SentPacket) -> bool {
        self.arrival_time == other.arrival_time
    }
}

#[derive(Debug, Default, Copy, Clone)]
pub struct DummySocketConfig {
    // Probability (between 0 and 1) of a packet being lost
    pub packet_loss: f64,
    // Minimal time before the packet is received
    pub latency: Duration,
    // Maximum extra random time before the packet is received
    pub max_jitter: Duration,
}

#[allow(dead_code)]
pub const NO_LOSS_CONFIG: DummySocketConfig = DummySocketConfig {
    packet_loss: 0.0,
    latency: Duration::from_millis(30),
    max_jitter: Duration::from_millis(30),
};  

pub struct DummySocket {
    addr: SocketAddr,
    packet_loss: f64,
    packet_loss_dist: Uniform<f64>,
    delay_dist: Uniform<f64>,
}

impl DummySocket {
    pub fn new(addr: SocketAddr, config: DummySocketConfig) -> Self {
        let latency = config.latency.as_secs_f64();
        let max_jitter = config.max_jitter.as_secs_f64();
        Self {
            addr,
            packet_loss: config.packet_loss,
            packet_loss_dist: Uniform::new_inclusive(0.0, 1.0),
            delay_dist: Uniform::new_inclusive(latency, latency + max_jitter),
        }
    }
}

impl Socket for DummySocket {
    fn receive(&mut self, buf: &mut [u8]) -> Option<(usize, SocketAddr)> {
        let mut map = PACKET_QUEUES.lock().unwrap();
        if let Some(packet_queue) = map.get_mut(&self.addr) {
            if let Some(next_message) = packet_queue.peek_mut() {
                if next_message.arrival_time < Instant::now() {
                    return {
                        let msg = std::collections::binary_heap::PeekMut::pop(next_message);
                        let data_len = msg.data.len();
                        buf[..data_len].copy_from_slice(&msg.data);
                        Some((data_len, msg.sender))
                    };
                }
            } 
        }
        None
    }

    fn send(&mut self, buf: &[u8], addr: SocketAddr) -> Option<()> {
        let mut rng = thread_rng();
        if self.packet_loss_dist.sample(&mut rng) >= self.packet_loss {
            PACKET_QUEUES.lock().unwrap().entry(addr).or_default().push(SentPacket {
                arrival_time: Instant::now() + Duration::from_secs_f64(self.delay_dist.sample(&mut rng)),
                sender: self.addr,
                data: Vec::from(buf),
            });
        }
        Some(())
    }
    
}

// TODO: allow sending packets from unknown sources
