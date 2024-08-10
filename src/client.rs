use crate::loss_simulation::LossSimulation;
use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};
use std::path::PathBuf;

pub struct Client {
    #[allow(dead_code)]
    loss_sim: Option<LossSimulation>,
}

impl Client {
    pub fn new(loss_sim: Option<LossSimulation>) -> Self {
        Client { loss_sim }
    }

    pub fn run(&self, host: Ipv4Addr, port: u16, _files: Vec<PathBuf>) {
        let socket = UdpSocket::bind("0.0.0.0:0").expect("Failed to bind socket");
        socket
            .connect(SocketAddrV4::new(host, port))
            .expect("Failed to connect to server");
        dbg!(&socket);
        socket.send(b"Hello, server!").expect("Failed to send data");
    }
}
