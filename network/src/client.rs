use std::time::Instant;
use super::packet::{serialize_packet, deserialize_packet};
use super::socket::{Socket, SocketAddr};
use super::types::*;

#[derive(Debug)]
enum Status {
    ConnectSent {
        client_salt: Salt,
        time: Instant,
    },
    ChallengeResponseSent {
        salts_xor: Salt,
        time: Instant,
    },
    Connected {
        salts_xor: Salt,
        last_server_packet: Instant,
    },
    Disconnected {
        message: String,
    },
}

pub struct Client<S: Socket> {
    server_addr: SocketAddr,
    socket: S,
    status: Status,
    buf: Vec<u8>,
}

impl<S: Socket> Client<S> {
    pub fn new(socket: S, server_addr: SocketAddr) -> Self {
        Self {
            server_addr,
            socket,
            status: Status::Disconnected { message: "Not yet connected".to_owned() },
            buf: Vec::with_capacity(MAX_PACKET_SIZE),
        }
    }

    pub fn connect(&mut self) {
        match &self.status {
            Status::Disconnected { .. } => {
                let client_salt = rand::random();
                self.status = Status::ConnectSent { client_salt, time: Instant::now() };
            }
            _ => {}
        }
    }

    pub fn read(&mut self) {
        while let Some((packet_size, src)) = {
            self.buf.resize(MAX_PACKET_SIZE, 0);
            self.socket.receive(&mut self.buf)
        } {
            if src != self.server_addr { continue; }
            if let Ok(packet) = deserialize_packet(&mut self.buf[0..packet_size]) {
                match self.status {
                    Status::ConnectSent { client_salt, .. } => {
                        // Did we receive the challenge ?
                        match packet {
                            ToClientPacket::Challenge { client_salt: packet_client_salt, server_salt } => {
                                if client_salt == packet_client_salt {
                                    self.status = Status::ChallengeResponseSent {
                                        salts_xor: client_salt ^ server_salt,
                                        time: Instant::now(),
                                    }
                                }
                            }
                            ToClientPacket::Disconnect { salts_xor, message } => {
                                if client_salt == salts_xor {
                                    self.status = Status::Disconnected { message };
                                }
                            }
                            _ => {}
                        }
                    }
                    Status::ChallengeResponseSent { salts_xor, .. } | Status::Connected { salts_xor, .. } => {
                        // Did we receive a normal message ?
                        match packet {
                            ToClientPacket::Message { salts_xor: message_salts_xor } => {
                                if salts_xor == message_salts_xor {
                                    self.status = Status::Connected { salts_xor, last_server_packet: Instant::now() };
                                    // TODO: handle message
                                    log::debug!("Client received message!");
                                }
                            }
                            ToClientPacket::Disconnect { salts_xor: message_salts_xor, message } => {
                                if salts_xor == message_salts_xor {
                                    self.status = Status::Disconnected { message };
                                }
                            }
                            _ => {}
                        }
                    }
                    Status::Disconnected { .. } => {}
                }
            }
        }
    }

    pub fn tick(&mut self) {
        self.read();

        match &self.status {
            Status::ConnectSent {
                client_salt,
                time,
            } => {
                // Timeout
                if Instant::now() - *time > DISCONNECT_TIMEOUT {
                    self.status = Status::Disconnected { message: TIMEOUT_MESSAGE.to_owned() };
                    return;
                }
                // Send connect packet
                let connect_packet = ToServerPacket::TryConnect { client_salt: *client_salt, padding: Default::default() };
                serialize_packet(&mut self.buf, &connect_packet).expect("Failed to serialize TryConnect packet");
                self.socket.send(&mut self.buf, self.server_addr);
            }
            Status::ChallengeResponseSent {
                salts_xor,
                time,
            } => {
                // Timeout
                if Instant::now() - *time > DISCONNECT_TIMEOUT {
                    self.status = Status::Disconnected { message: TIMEOUT_MESSAGE.to_owned() };
                    return;
                }
                // Send challenge response packet
                let connect_packet = ToServerPacket::ChallengeResponse { salts_xor: *salts_xor, padding: Default::default() };
                serialize_packet(&mut self.buf, &connect_packet).expect("Failed to serialize ChallengeResponse packet");
                self.socket.send(&mut self.buf, self.server_addr);
            }
            Status::Connected { last_server_packet, .. } => {
                // Timeout
                if Instant::now() - *last_server_packet > DISCONNECT_TIMEOUT {
                    self.status = Status::Disconnected { message: TIMEOUT_MESSAGE.to_owned() };
                    return;
                }
                // TODO: send messages
            }
            Status::Disconnected {..} => {}
        }
    }
}