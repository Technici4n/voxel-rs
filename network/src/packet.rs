use bincode::{DefaultOptions, Options};
use crc::crc32;
use serde::{Serialize, de::DeserializeOwned};
use super::types::*;

lazy_static::lazy_static! {
    static ref BINCODE_OPTIONS: bincode::config::WithOtherLimit<DefaultOptions, bincode::config::Bounded> = {
        DefaultOptions::default().with_limit(MAX_PACKET_CONTENT as u64)
    };
}

pub fn serialize_packet<P: Serialize>(target: &mut Vec<u8>, packet: &P) -> bincode::Result<()> {
    // Resize the buffer
    let content_size = BINCODE_OPTIONS.serialized_size(packet)? as usize;
    if content_size > MAX_PACKET_CONTENT {
        return Err(Box::new(bincode::ErrorKind::SizeLimit));
    }
    target.resize(content_size as usize + HEADER_SIZE, 0);
    // Serialize the packet
    BINCODE_OPTIONS.serialize_into(&mut target[HEADER_SIZE..], packet)?;
    // Compute the checksum (including magic number)
    for i in 0..4 {
        target[i] = MAGIC_NUMBER[i];
    }
    let checksum = crc32::checksum_ieee(&target[..]).to_le_bytes();
    // Store the checksum
    for i in 0..4 {
        target[i] = checksum[i];
    }
    Ok(())
}

pub fn deserialize_packet<P: DeserializeOwned>(source: &mut [u8]) -> bincode::Result<P> {
    // Check size
    let packet_size = source.len();
    if packet_size < HEADER_SIZE {
        return Err(Box::new(bincode::ErrorKind::SizeLimit));
    }
    let content_size = packet_size - HEADER_SIZE;
    if content_size > MAX_PACKET_CONTENT {
        return Err(Box::new(bincode::ErrorKind::SizeLimit));
    }
    // Check checksum
    let mut packet_checksum: [u8; 4] = Default::default();
    for i in 0..4 {
        packet_checksum[i] = source[i];
        source[i] = MAGIC_NUMBER[i];
    }
    let checksum = crc32::checksum_ieee(&source[..]).to_le_bytes();
    for i in 0..4 {
        source[i] = packet_checksum[i];
    }
    for i in 0..4 {
        if checksum[i] != packet_checksum[i] {
            return Err(Box::new(bincode::ErrorKind::DeserializeAnyNotSupported)); // Actually, invalid checksum
        }
    }
    BINCODE_OPTIONS.deserialize_from(&source[HEADER_SIZE..])
}


#[test]
fn test_ser_de() {
    let msg1 = ToServerPacket::Message {
        salts_xor: 1194876546,
        messages: vec![
            Message::ReliableAcks {
                first_sequence: 0,
                acks: BitSet::new().into(),
            },
        ]
    };
    let mut v = Vec::new();
    serialize_packet(&mut v, &msg1).unwrap();
    let msg2 = deserialize_packet(&mut v[..]).unwrap();
    assert_eq!(msg1, msg2);
}
