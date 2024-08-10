use crate::loss_simulation::LossSimulation;
use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};
use std::str::from_utf8;

pub struct Server {
    port: u16,
    #[allow(dead_code)]
    loss_sim: Option<LossSimulation>,
}

impl Server {
    pub fn new(port: u16, loss_sim: Option<LossSimulation>) -> Self {
        Server { port, loss_sim }
    }

    pub fn run(&self) {
        let socket = UdpSocket::bind(SocketAddrV4::new(
            Ipv4Addr::new(0, 0, 0, 0),
            self.port,
        ))
        .expect("Failed to bind socket");
        dbg!(&socket);
        let mut buf = [0; 1024];
        loop {
            let size = match socket.recv(&mut buf) {
                Ok(size) => size,
                Err(e) => {
                    eprintln!("Failed to receive data: {}", e);
                    continue;
                }
            };
            let message = from_utf8(&buf[..size]).unwrap();
            dbg!(message);
        }
    }
}
