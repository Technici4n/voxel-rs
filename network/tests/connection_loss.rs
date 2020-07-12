use std::str::FromStr;
use std::thread;
use std::time::Duration;
use voxel_rs_network::{Client, Server, ServerEvent, SocketAddr, MessageDelivery};

mod common;
use self::common::{DummySocket, DummySocketConfig};

// Server sends 42 to client, client sends back 43 to server, ordered
#[test]
fn test_connection_with_loss() {
    let config = DummySocketConfig {
        packet_loss: 0.8,
        latency: Duration::from_millis(100),
        max_jitter: Duration::from_millis(100),
    };
    let sleep_duration = Duration::from_millis(20);
    let client_addr = SocketAddr::from_str("127.0.0.1:42").unwrap();
    let server_addr = SocketAddr::from_str("127.0.0.1:43").unwrap();
    thread::spawn(move || {
        let client_socket = DummySocket::new(client_addr, config);
        let mut client = Client::new(client_socket, server_addr);
        client.connect();

        loop {
            client.tick();
            let mut send_back = false;
            for message in client.get_messages() {
                if message.1 == vec![42] {
                    send_back = true;
                }
            }
            if send_back {
                client.send_message(vec![43], MessageDelivery::Ordered);
            }
            thread::sleep(sleep_duration);
        }
    });

    let server_thread = thread::spawn(move || {
        let server_socket = DummySocket::new(server_addr, config);
        let mut server = Server::new(server_socket);

        loop {
            server.tick();
            let mut send_back_id = None;
            for event in server.get_events() {
                match event {
                    ServerEvent::Connected { id } => {
                        send_back_id = Some(id);
                    }
                    ServerEvent::Message { data, .. } => {
                        if data == vec![43] {
                            return true;
                        }
                    }
                    _ => {}
                }
            }
            if let Some(id) = send_back_id {
                server.send_message(id, vec![42], MessageDelivery::Ordered);
            }
            thread::sleep(sleep_duration);
        }
    });

    let join_result = server_thread.join();
    assert!(join_result.unwrap(), "Server received the client's message");
}