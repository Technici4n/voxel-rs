use serde::{Serialize, Deserialize};
use std::time::Duration;

pub type Salt = u32;

pub const MAGIC_NUMBER: [u8; 4] = 0x4212313fu32.to_le_bytes();
pub const MAX_PACKET_SIZE: usize = 1200;
pub const HEADER_SIZE: usize = 4; // only CRC32
pub const MAX_PACKET_CONTENT: usize = MAX_PACKET_SIZE - HEADER_SIZE;
pub const DISCONNECT_TIMEOUT: Duration = Duration::from_secs(5);
pub const TIMEOUT_MESSAGE: &'static str = "Timed out";

#[derive(Debug, Serialize, Deserialize)]
pub enum ToClientPacket {
    Challenge { client_salt: Salt, server_salt: Salt },
    Message { salts_xor: Salt },
    Disconnect { salts_xor: Salt, message: String }, // salts_xor is just the client salt if the server is full
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ToServerPacket {
    TryConnect { client_salt: Salt, padding: [[u8; 32]; 32] },
    ChallengeResponse { salts_xor: Salt, padding: [[u8; 32]; 32] },
    Message { salts_xor: Salt },
    Disconnect { salts_xor: Salt },
}