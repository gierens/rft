use crate::loss_simulation::LossSimulation;
use crate::wire::Packet;
use std::net::{Ipv4Addr, SocketAddrV4};
use tokio::net::UdpSocket;

pub struct Server {
    port: u16,
    #[allow(dead_code)]
    loss_sim: Option<LossSimulation>,
}

impl Server {
    pub fn new(port: u16, loss_sim: Option<LossSimulation>) -> Self {
        Server { port, loss_sim }
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        //HashMap for client IPs
        //TODO: map connectionID -> IP:Port

        //HashMap for connection handlers
        //TODO: map connectionID -> mpsc::Sender

        //mpsc channel <Packet>: handler output -> transmitter input
        //TODO

        let socket = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), self.port))
            .await
            .expect("Failed to bind socket");
        let mut buf = [0; 1024];

        //
        loop {
            let size = socket.recv(&mut buf).await?;
            let packet = Packet::parse_buf(&buf[..size]).expect("Failed to parse packet");
            dbg!(packet);
        }
    }
}
