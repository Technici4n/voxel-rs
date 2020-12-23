use std::{
    collections::VecDeque,
    time::Instant,
};
use super::types::*;

struct QueuedPacket {
    pub sequence: Sequence,
    pub data: Vec<u8>,
    pub first_send: Option<Instant>,
    pub last_send: Instant,
}

pub struct Sender {
    /// Queued reliable packets
    reliable_packets: VecDeque<QueuedPacket>,
    /// Sequence for the next packet
    next_sequence: Sequence,
    /// Earliest sequence number that the receiver hasn't acked yet
    earliest_unacked_sequence: Sequence,
}

/// First receive, then get_message, then get_acks
pub struct Receiver {
    received: Vec<Option<Vec<u8>>>,
    received_sequences: [Sequence; RELIABLE_BUFFER_SIZE],
    next_sequence: Sequence,
}

impl Sender {
    pub fn new() -> Self {
        Self {
            reliable_packets: VecDeque::new(),
            next_sequence: 1,
            earliest_unacked_sequence: 1,
        }
    }

    pub fn send(&mut self, data: Vec<u8>) {
        self.reliable_packets.push_back(QueuedPacket {
            sequence: { (self.next_sequence, self.next_sequence += 1).0 },
            data,
            first_send: None,
            last_send: Instant::now() - RESEND_DELAY,
        });
    }

    // True if sent, false if bandwidth is exceeded
    pub fn tick<F: FnMut(Message) -> bool>(&mut self, mut send_message: F) {
        let max_sequence = self.earliest_unacked_sequence + RELIABLE_BUFFER_SIZE as u32;
        for packet in self.reliable_packets.iter_mut() {
            // Don't send a packet the receiver can't buffer
            if packet.sequence >= max_sequence {
                break
            }
            // Resend packet if enough time has elapsed
            let now = Instant::now();
            if now - packet.last_send > RESEND_DELAY {
                if send_message(Message::Reliable {
                    sequence: packet.sequence,
                    data: packet.data.clone(),
                }) {
                    packet.last_send = now;
                    if packet.first_send.is_none() {
                        packet.first_send = Some(now);
                    }
                } else {
                    // Break if bandwidth is exceeded
                    break
                }
            }
        }
    }

    pub fn receive_acks(&mut self, first_sequence: Sequence, acks: BitSet) {
        // TODO: process time to estimate RTT and packet loss
        self.reliable_packets.retain(|packet| {
            if packet.sequence < first_sequence { false }
            else {
                let idx = packet.sequence - first_sequence;
                if idx as usize >= acks.len() { true }
                else { acks[idx as usize] }
            }
        });
        self.earliest_unacked_sequence = match self.reliable_packets.front() {
            Some(packet) => packet.sequence,
            None => self.next_sequence,
        };
    }
}

impl Receiver {
    pub fn new() -> Self {
        Self {
            received: vec![None; RELIABLE_BUFFER_SIZE],
            received_sequences: [0; RELIABLE_BUFFER_SIZE],
            next_sequence: 1,
        }
    }

    pub fn get_message(&mut self) -> Option<Vec<u8>> {
        let next_idx = self.next_sequence as usize % RELIABLE_BUFFER_SIZE;
        if self.received[next_idx].is_some() && self.received_sequences[next_idx] == self.next_sequence {
            self.next_sequence += 1;
            self.received[next_idx].take()
        } else {
            None
        }
    }

    pub fn receive(&mut self, sequence: Sequence, data: Vec<u8>) {
        let idx = sequence as usize % RELIABLE_BUFFER_SIZE;
        if sequence > self.received_sequences[idx] {
            assert!(sequence - self.received_sequences[idx] <= RELIABLE_BUFFER_SIZE as u32, "sequence number too high received");
            self.received_sequences[idx] = sequence;
            self.received[idx] = Some(data);
        }
    }

    pub fn get_acks(&self) -> (Sequence, BitSet) {
        let seq = self.next_sequence;
        let mut set = BitSet::with_capacity(RELIABLE_BUFFER_SIZE);
        for i in 0..RELIABLE_BUFFER_SIZE {
            let idx = (i + seq as usize) % RELIABLE_BUFFER_SIZE;
            set.push(self.received_sequences[idx] >= seq && self.received[idx].is_some());
        }
        // Remove final 0s
        while let Some(last_bit) = set.iter().by_val().last() {
            if !last_bit {
                set.pop().unwrap();
            }
        }
        (seq, set)
    }
}
