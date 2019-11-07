use crate::player::PlayerId;

pub mod messages;

/// An event that the server received.
#[derive(Debug, Clone)]
pub enum ServerEvent {
    /// No pending events.
    NoEvent,
    /// Client with given id connected.
    ClientConnected(PlayerId),
    /// Client with given id disconnected.
    ClientDisconnected(PlayerId),
    /// Client with given id sent a message.
    ClientMessage(PlayerId, messages::ToServer),
}

/// An event that the client received.
#[derive(Debug, Clone)]
pub enum ClientEvent {
    /// No pending events.
    NoEvent,
    /// Connected to the server.
    Connected,
    /// Disconnected from the server.
    Disconnected,
    /// Server sent a message.
    ServerMessage(messages::ToClient),
}

/// An abstraction over a network server.
pub trait Server {
    /// Receive the next event.
    fn receive_event(&mut self) -> ServerEvent;
    /// Send a message to a client. The message will be dropped if it can't be sent.
    fn send(&mut self, client: PlayerId, message: messages::ToClient);
}

/// An abstraction over a network client.
pub trait Client {
    /// Receive the next event
    fn receive_event(&mut self) -> ClientEvent;
    /// Send a message to the server. The message will be dropped if it can't be sent.
    fn send(&mut self, message: messages::ToServer);
}

/// Dummy client and server implementations for testing
pub mod dummy;
