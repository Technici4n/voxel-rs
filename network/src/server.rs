use std::time::Instant;
use super::packet::{serialize_packet, deserialize_packet};
use super::socket::{Socket, SocketAddr};
use super::types::*;

const MAX_PLAYERS: usize = 10;

enum ClientSlot {
    Empty,
    ConnectReceived {
        client_salt: Salt,
        server_salt: Salt,
        time: Instant,
        remote: SocketAddr,
    },
    Connected {
        salts_xor: Salt,
        last_client_packet: Instant,
        remote: SocketAddr,
    },
}

impl Default for ClientSlot {
    fn default() -> Self {
        Self::Empty
    }
}

pub struct Server<S: Socket> {
    socket: S,
    players: [ClientSlot; MAX_PLAYERS],
    buf: Vec<u8>,
}

impl<S: Socket> Server<S> {
    pub fn new(socket: S) -> Server<S> {
        Self {
            socket,
            players: Default::default(),
            buf: Vec::with_capacity(MAX_PACKET_SIZE),
        }
    }

    pub fn read(&mut self) {
        while let Some((packet_size, src)) = {
            self.buf.resize(MAX_PACKET_SIZE, 0);
            self.socket.receive(&mut self.buf)
        } {
            let packet = match deserialize_packet(&mut self.buf[0..packet_size]) {
                Ok(packet) => packet,
                Err(_) => continue,
            };
            if let Some(i) = self.find_client_slot(src) {
                match &self.players[i] {
                    &ClientSlot::Empty => unreachable!("Logic error: empty slot can't be a client slot"),
                    &ClientSlot::ConnectReceived { client_salt, server_salt , .. } => {
                        match packet {
                            ToServerPacket::ChallengeResponse { salts_xor: packet_salts_xor, .. } => {
                                if client_salt ^ server_salt == packet_salts_xor {
                                    self.players[i] = ClientSlot::Connected {
                                        salts_xor: client_salt ^ server_salt,
                                        last_client_packet: Instant::now(),
                                        remote: src,
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    &ClientSlot::Connected { salts_xor, .. } => {
                        match packet {
                            ToServerPacket::Message { salts_xor: packet_salts_xor, .. } => {
                                if salts_xor == packet_salts_xor {
                                    // TODO: handle message
                                    log::debug!("Server received message!");
                                }
                            }
                            ToServerPacket::Disconnect { salts_xor: packet_salts_xor, .. } => {
                                if salts_xor == packet_salts_xor {
                                    self.players[i] = ClientSlot::Empty;
                                }
                            }
                            _ => {}
                        }
                    }
                }
            } else if let Some(i) = self.find_free_slot() {
                match packet {
                    ToServerPacket::TryConnect { client_salt, .. } => {
                        let server_salt: Salt = rand::random();
                        self.players[i] = ClientSlot::ConnectReceived {
                            client_salt,
                            server_salt,
                            time: Instant::now(),
                            remote: src,
                        }
                    }
                    _ => {}
                }
            } else {
                // TODO: send 10x server full message ?
            }
        }
    }

    fn find_client_slot(&self, addr: SocketAddr) -> Option<usize> {
        for (i, slot) in self.players.iter().enumerate() {
            match slot {
                &ClientSlot::ConnectReceived { remote, .. } | &ClientSlot::Connected { remote, .. } => {
                    if remote == addr {
                        return Some(i);
                    }
                }
                _ => {}
            }
        }
        None
    }

    fn find_free_slot(&self) -> Option<usize> {
        for (i, slot) in self.players.iter().enumerate() {
            if let &ClientSlot::Empty = slot {
                return Some(i);
            }
        }
        None
    }

    pub fn tick(&mut self) {
        self.read();

        for slot in self.players.iter_mut() {
            match slot {
                ClientSlot::Empty => {}
                ClientSlot::ConnectReceived { client_salt, server_salt, time, remote } => {
                    // Timeout
                    if Instant::now() - *time > DISCONNECT_TIMEOUT {
                        *slot = ClientSlot::Empty {};
                        return;
                    }
                    // Send challenge packet
                    let challenge_packet = ToClientPacket::Challenge { client_salt: *client_salt, server_salt: *server_salt };
                    serialize_packet(&mut self.buf, &challenge_packet).expect("Failed to serialize Challenge packet");
                    self.socket.send(&mut self.buf, *remote);
                }
                ClientSlot::Connected { last_client_packet, .. } => {
                    // Timeout
                    if Instant::now() - *last_client_packet > DISCONNECT_TIMEOUT {
                        *slot = ClientSlot::Empty {};
                        return;
                    }
                    // TODO: send messages
                }
            }
        }
    }
}