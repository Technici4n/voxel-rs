pub use std::net::{UdpSocket, SocketAddr};

// TODO: handle errors :-)
pub trait Socket {
    /// Receive a packet. Return the number of bytes read and the origin.
    fn receive(&mut self, buf: &mut [u8]) -> Option<(usize, SocketAddr)>;
    /// Send a packet.
    fn send(&mut self, buf: &[u8], addr: SocketAddr) -> Option<()>;
}

impl Socket for UdpSocket {
    fn receive(&mut self, buf: &mut [u8]) -> Option<(usize, SocketAddr)> {
        self.recv_from(buf).ok()
    }
    fn send(&mut self, buf: &[u8], addr: SocketAddr) -> Option<()> {
        let res = self.send_to(buf, addr);
        match res {
            Ok(bytes_read) => if buf.len() == bytes_read {
                Some(())
            } else {
                None
            }
            Err(e) => {
                log::warn!("Packet sending error: {:?}", e);
                None
            }
        }
    }
}