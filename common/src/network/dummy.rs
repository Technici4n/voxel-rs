use super::messages::{ToClient, ToServer};
use crate::{
    network::{ClientEvent, ServerEvent},
    player::PlayerId,
};
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};

pub struct DummyClient {
    first_queried: bool,
    pub(self) to_server: Sender<ToServer>,
    pub(self) to_client: Receiver<ToClient>,
}

pub struct DummyServer {
    first_queried: bool,
    pub(self) to_client: Sender<ToClient>,
    pub(self) to_server: Receiver<ToServer>,
}

pub fn new() -> (DummyClient, DummyServer) {
    let server_to_client = channel();
    let client_to_server = channel();
    (
        DummyClient {
            first_queried: true,
            to_server: client_to_server.0,
            to_client: server_to_client.1,
        },
        DummyServer {
            first_queried: true,
            to_client: server_to_client.0,
            to_server: client_to_server.1,
        },
    )
}

impl super::Server for DummyServer {
    fn receive_event(&mut self) -> ServerEvent {
        if self.first_queried {
            self.first_queried = false;
            return ServerEvent::ClientConnected(PlayerId(0));
        }
        match self.to_server.try_recv() {
            Ok(m) => ServerEvent::ClientMessage(PlayerId(0), m),
            Err(TryRecvError::Empty) => ServerEvent::NoEvent,
            Err(TryRecvError::Disconnected) => panic!("Got to somehow terminate the server :)"),
        }
    }

    fn send(&mut self, _: PlayerId, message: ToClient) {
        self.to_client.send(message).unwrap();
    }
}

impl super::Client for DummyClient {
    fn receive_event(&mut self) -> ClientEvent {
        if self.first_queried {
            self.first_queried = false;
            return ClientEvent::Connected;
        }
        match self.to_client.try_recv() {
            Ok(m) => ClientEvent::ServerMessage(m),
            Err(TryRecvError::Empty) => ClientEvent::NoEvent,
            Err(TryRecvError::Disconnected) => unreachable!(),
        }
    }

    fn send(&mut self, message: ToServer) {
        self.to_server.send(message).unwrap();
    }
}
