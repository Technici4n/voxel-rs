use std::time::Instant;
use super::channel::{Sender, Receiver};
use super::packet::{serialize_packet, deserialize_packet};
use super::socket::{Socket, SocketAddr};
use super::types::*;

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
        sender: Sender,
        receiver: Receiver,
        pending_unreliable: Vec<Vec<u8>>,
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
    messages: Vec<(MessageDelivery, Vec<u8>)>,
}

impl<S: Socket> Client<S> {
    pub fn new(socket: S, server_addr: SocketAddr) -> Self {
        Self {
            server_addr,
            socket,
            status: Status::Disconnected { message: "Not yet connected".to_owned() },
            buf: Vec::with_capacity(MAX_PACKET_SIZE),
            messages: Vec::new(),
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

    pub fn is_connected(&self) -> bool {
        if let Status::Connected { .. } = self.status {
            true
        } else {
            false
        }
    }

    pub fn read(&mut self) {
        while let Some((packet_size, src)) = {
            self.buf.resize(MAX_PACKET_SIZE, 0);
            self.socket.receive(&mut self.buf)
        } {
            if src != self.server_addr { continue; }
            if let Ok(packet) = deserialize_packet(&mut self.buf[0..packet_size]) {
                match &mut self.status {
                    Status::ConnectSent { client_salt, .. } => {
                        // Did we receive the challenge ?
                        match packet {
                            ToClientPacket::Challenge { client_salt: packet_client_salt, server_salt } => {
                                if *client_salt == packet_client_salt {
                                    self.status = Status::ChallengeResponseSent {
                                        salts_xor: *client_salt ^ server_salt,
                                        time: Instant::now(),
                                    };
                                }
                            }
                            ToClientPacket::Disconnect { salts_xor, message } => {
                                if *client_salt == salts_xor {
                                    self.status = Status::Disconnected { message };
                                }
                            }
                            _ => {}
                        }
                    }
                    Status::ChallengeResponseSent { salts_xor, .. } | Status::Connected { salts_xor, .. } => {
                        // Did we receive a normal message ?
                        match packet {
                            ToClientPacket::Message { salts_xor: message_salts_xor, messages } => {
                                if *salts_xor == message_salts_xor {
                                    match self.status {
                                        Status::ChallengeResponseSent { salts_xor, .. } => {
                                            self.status = Status::Connected { 
                                                salts_xor,
                                                last_server_packet: Instant::now(),
                                                sender: Sender::new(),
                                                receiver: Receiver::new(),
                                                pending_unreliable: Vec::new(),
                                            };
                                        }
                                        _ => {}
                                    }
                                    if let Status::Connected { sender, receiver, .. } = &mut self.status {
                                        for msg in messages {
                                            match msg {
                                                Message::Unreliable(data) => self.messages.push((MessageDelivery::Unreliable, data)),
                                                Message::Reliable { sequence, data } => receiver.receive(sequence, data),
                                                Message::ReliableAcks { first_sequence, acks } => sender.receive_acks(first_sequence, acks.into()),
                                            }
                                        }
                                        while let Some(data) = receiver.get_message() {
                                            self.messages.push((MessageDelivery::Ordered, data));
                                        }
                                    }
                                }
                            }
                            ToClientPacket::Disconnect { salts_xor: message_salts_xor, message } => {
                                if *salts_xor == message_salts_xor {
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

        match &mut self.status {
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
            Status::Connected { last_server_packet, salts_xor, pending_unreliable, sender, receiver, .. } => {
                // Timeout
                if Instant::now() - *last_server_packet > DISCONNECT_TIMEOUT {
                    self.status = Status::Disconnected { message: TIMEOUT_MESSAGE.to_owned() };
                    return;
                }
                let Self { buf, socket, server_addr, .. } = self;
                let mut packet_body: Vec<Message> = Vec::new();
                let mut send_message = |message| {
                    packet_body.push(message);
                    let mut packet = ToServerPacket::Message {
                        salts_xor: *salts_xor,
                        messages: std::mem::replace(&mut packet_body, Vec::new()),
                    };
                    // If the new message can't fit in the packet, then send the packet without the new message
                    // TODO: maybe optimize ?
                    if serialize_packet(buf, &packet).is_err() {
                        // Extract last message
                        let message = match &mut packet {
                            ToServerPacket::Message { messages, .. } => messages,
                            _ => unreachable!(),
                        }.pop().unwrap();
                        // Send packet
                        serialize_packet(buf, &packet).expect("Failed to serialize packet to server");
                        socket.send(buf, *server_addr);
                        // Prepare next packet
                        packet_body.push(message);
                    } else {
                        match packet {
                            ToServerPacket::Message { messages, .. } => packet_body = messages,
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
                    let packet = ToServerPacket::Message {
                        salts_xor: *salts_xor,
                        messages: packet_body,
                    };
                    serialize_packet(&mut self.buf, &packet).expect("Failed to serialize packet to server");
                    self.socket.send(&mut self.buf, *server_addr);
                }
            }
            Status::Disconnected {..} => {}
        }
    }

    pub fn send_message(&mut self, data: Vec<u8>, delivery: MessageDelivery) {
        if let Status::Connected { sender, pending_unreliable, .. } = &mut self.status {
            match delivery {
                MessageDelivery::Unreliable => pending_unreliable.push(data),
                MessageDelivery::Ordered => sender.send(data),
            }
        }
    }

    pub fn get_messages<'a>(&'a mut self) -> impl 'a + Iterator<Item = (MessageDelivery, Vec<u8>)> {
        self.messages.drain(..)
    }
}