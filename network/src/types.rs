use bitvec::prelude::*;
use serde::{Serialize, Deserialize};
use std::time::Duration;

pub type Salt = u32;
pub type BitSet = BitVec<Lsb0, u8>;
pub type Sequence = u32;

pub const MAGIC_NUMBER: [u8; 4] = 0x4212313fu32.to_le_bytes();
pub const MAX_PACKET_SIZE: usize = 1200;
pub const HEADER_SIZE: usize = 4; // only CRC32
pub const MAX_PACKET_CONTENT: usize = MAX_PACKET_SIZE - HEADER_SIZE;
pub const DISCONNECT_TIMEOUT: Duration = Duration::from_secs(5);
pub const TIMEOUT_MESSAGE: &'static str = "Timed out";
pub const RELIABLE_BUFFER_SIZE: usize = 1024;
pub const RESEND_DELAY: Duration = Duration::from_millis(100);

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ToClientPacket {
    Challenge { client_salt: Salt, server_salt: Salt },
    Message { salts_xor: Salt, messages: Vec<Message> },
    Disconnect { salts_xor: Salt, message: String }, // salts_xor is just the client salt if the server is full
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ToServerPacket {
    TryConnect { client_salt: Salt, padding: [[u8; 32]; 32] },
    ChallengeResponse { salts_xor: Salt, padding: [[u8; 32]; 32] },
    Message { salts_xor: Salt, messages: Vec<Message> },
    Disconnect { salts_xor: Salt },
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Message {
    /// Unreliable message
    Unreliable(Vec<u8>),
    /// Reliable message
    Reliable {
        sequence: Sequence,
        data: Vec<u8>,
    },
    /// Acks for reliable messages
    /// The i-th bit in `acks` is 1 if the message with sequence number `first_sequence + i` was received, and 0 otherwise.
    ReliableAcks {
        first_sequence: Sequence,
        acks: SimpleBitSet,
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum MessageDelivery {
    /// Message may not arrive.
    Unreliable,
    /// The message is guaranteed to arrive exactly once in order (with respect to the other Ordered messages).
    Ordered,
}

// For easier serialization
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SimpleBitSet {
    last_byte_bits: u8,
    bytes: Vec<u8>,
}

impl From<BitSet> for SimpleBitSet {
    fn from(bs: BitSet) -> Self {
        Self {
            last_byte_bits: (bs.len() % 8) as u8,
            bytes: bs.into(),
        }
    }
}

impl Into<BitSet> for SimpleBitSet {
    fn into(self) -> BitSet {
        let mut bs = BitSet::from_vec(self.bytes);
        while bs.len() % 8 > self.last_byte_bits as usize {
            bs.pop();
        }
        bs
    }
}
