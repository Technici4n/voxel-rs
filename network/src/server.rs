use std::time::Instant;
use super::channel::{Sender, Receiver};
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
        sender: Sender,
        receiver: Receiver,
        pending_unreliable: Vec<Vec<u8>>,
    },
}

impl Default for ClientSlot {
    fn default() -> Self {
        Self::Empty
    }
}

pub enum ServerEvent {
    Connected { id: SocketAddr },
    Disconnected { id: SocketAddr },
    Message { source_id: SocketAddr, kind: MessageDelivery, data: Vec<u8> },
}

/// Send messages then tick
pub struct Server<S: Socket> {
    socket: S,
    players: [ClientSlot; MAX_PLAYERS],
    buf: Vec<u8>,
    events: Vec<ServerEvent>,
}

impl<S: Socket> Server<S> {
    pub fn new(socket: S) -> Server<S> {
        Self {
            socket,
            players: Default::default(),
            buf: Vec::with_capacity(MAX_PACKET_SIZE),
            events: Vec::new(),
        }
    }

    pub fn read(&mut self) {
        while let Some((packet_size, src)) = {
            self.buf.resize(MAX_PACKET_SIZE, 0);
            self.socket.receive(&mut self.buf)
        } {
            let packet = match deserialize_packet(&mut self.buf[0..packet_size]) {
                Ok(packet) => packet,
                Err(e) => {
                    dbg!(&mut self.buf[0..packet_size]);
                    dbg!(e);
                    continue
                },
            };
            if let Some(i) = self.find_client_slot(src) {
                match &mut self.players[i] {
                    &mut ClientSlot::Empty => unreachable!("Logic error: empty slot can't be a client slot"),
                    &mut ClientSlot::ConnectReceived { client_salt, server_salt , .. } => {
                        match packet {
                            ToServerPacket::ChallengeResponse { salts_xor: packet_salts_xor, .. } => {
                                if client_salt ^ server_salt == packet_salts_xor {
                                    self.players[i] = ClientSlot::Connected {
                                        salts_xor: client_salt ^ server_salt,
                                        last_client_packet: Instant::now(),
                                        remote: src,
                                        sender: Sender::new(),
                                        receiver: Receiver::new(),
                                        pending_unreliable: Vec::new(),
                                    };
                                    self.events.push(ServerEvent::Connected { id: src });
                                }
                            }
                            _ => {}
                        }
                    }
                    &mut ClientSlot::Connected { salts_xor, ref mut sender, ref mut receiver, .. } => {
                        match packet {
                            ToServerPacket::Message { salts_xor: packet_salts_xor, messages } => {
                                if salts_xor == packet_salts_xor {
                                    for message in messages {
                                        match message {
                                            Message::Unreliable(data) => self.events.push(ServerEvent::Message {
                                                source_id: src,
                                                kind: MessageDelivery::Unreliable,
                                                data,
                                            }),
                                            Message::Reliable { sequence, data } => receiver.receive(sequence, data),
                                            Message::ReliableAcks { first_sequence, acks } => sender.receive_acks(first_sequence, acks.into()),
                                        }
                                    }
                                    while let Some(data) = receiver.get_message() {
                                        self.events.push(ServerEvent::Message {
                                            source_id: src,
                                            kind: MessageDelivery::Ordered,
                                            data,
                                        });
                                    }
                                }
                            }
                            ToServerPacket::Disconnect { salts_xor: packet_salts_xor, .. } => {
                                if salts_xor == packet_salts_xor {
                                    self.players[i] = ClientSlot::Empty;
                                    self.events.push(ServerEvent::Disconnected { id: src });
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
                ClientSlot::Connected { last_client_packet, salts_xor, remote, pending_unreliable, sender, receiver, .. } => {
                    // Timeout
                    if Instant::now() - *last_client_packet > DISCONNECT_TIMEOUT {
                        self.events.push(ServerEvent::Disconnected { id: *remote });
                        *slot = ClientSlot::Empty {};
                        return;
                    }
                    let Self { buf, socket, .. } = self;
                    let mut packet_body: Vec<Message> = Vec::new();
                    let mut send_message = |message| {
                        packet_body.push(message);
                        let mut packet = ToClientPacket::Message {
                            salts_xor: *salts_xor,
                            messages: std::mem::replace(&mut packet_body, Vec::new()),
                        };
                        // If the new message can't fit in the packet, then send the packet without the new message
                        // TODO: maybe optimize ?
                        if serialize_packet(buf, &packet).is_err() {
                            // Extract last message
                            let message = match &mut packet {
                                ToClientPacket::Message { messages, .. } => messages,
                                _ => unreachable!(),
                            }.pop().unwrap();
                            // Send packet
                            serialize_packet(buf, &packet).expect("Failed to serialize packet to client");
                            socket.send(buf, *remote);
                            // Prepare next packet
                            packet_body.push(message);
                        } else {
                            match packet {
                                ToClientPacket::Message { messages, .. } => packet_body = messages,
                                _ => unreachable!(),
                            }
                        }
                        // TODO: implement rate control
                        true
                    };
                    for message in pending_unreliable.drain(..) {
                        send_message(Message::Unreliable(message));
                    }
                    // Send acks
                    let (first_sequence, acks) = receiver.get_acks();
                    send_message(Message::ReliableAcks { first_sequence, acks: acks.into() });
                    // Send reliable messages
                    sender.tick(send_message);
                    // Send last buffered messages
                    if packet_body.len() > 0 {
                        let packet = ToClientPacket::Message {
                            salts_xor: *salts_xor,
                            messages: packet_body,
                        };
                        serialize_packet(&mut self.buf, &packet).expect("Failed to serialize packet to client");
                        self.socket.send(&mut self.buf, *remote);
                    }
                }
            }
        }
    }

    // TODO: implement rate control
    pub fn send_message(&mut self, addr: SocketAddr, data: Vec<u8>, delivery: MessageDelivery) {
        if let Some(slot) = self.find_client_slot(addr) {
            if let ClientSlot::Connected {
                sender,
                pending_unreliable,
                ..
            } = &mut self.players[slot] {
                match delivery {
                    MessageDelivery::Unreliable => {
                        pending_unreliable.push(data);
                    }
                    MessageDelivery::Ordered => {
                        sender.send(data);
                    }
                }
            }
        }
    }

    pub fn get_events<'a>(&'a mut self) -> impl 'a + Iterator<Item = ServerEvent> {
        self.events.drain(..)
    }
}