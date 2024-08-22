use crate::conn_handler::connection_handler;
use crate::loss_simulation::LossSimulation;
use crate::wire::Packet;
use futures::channel::mpsc;
use futures::SinkExt;
use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use tokio::net::UdpSocket;

pub struct Server {
    port: u16,
    #[allow(dead_code)]
    loss_sim: Option<LossSimulation>,
}

#[allow(dead_code)]
#[allow(unused_mut)]
#[allow(unused_variables)]
impl Server {
    pub fn new(port: u16, loss_sim: Option<LossSimulation>) -> Self {
        Server { port, loss_sim }
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        //HashMap for client IPs
        let mut output_map: HashMap<u32, SocketAddr> = HashMap::new();

        //HashMap for connection handlers
        let mut input_map: HashMap<u32, mpsc::Sender<Packet>> = HashMap::new();

        //mpsc channel <Packet>: handler output -> transmitter input
        let (mux_tx, mux_rx) = mpsc::channel(32);

        let socket = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), self.port))
            .await
            .expect("Failed to bind socket");
        let mut buf = [0; 1024];

        //TODO: delete closed connections from HashMaps

        //start packet switching task
        tokio::spawn(async move {
            let mut cid_ctr = 1u32;
            loop {
                let size = socket.recv(&mut buf).await.expect("Socket error");
                let packet = Packet::parse_buf(&buf[..size]).expect("Failed to parse packet");

                match packet.connection_id() {
                    0 => {
                        let (mut ctx, crx) = mpsc::channel(8);

                        ctx.send(packet).await.unwrap();

                        input_map.insert(cid_ctr, ctx);

                        let mux_tx_c = mux_tx.clone();
                        tokio::spawn(async move {
                            connection_handler(crx, mux_tx_c, cid_ctr)
                                .await
                                .expect("connection handler error");
                        });

                        cid_ctr += 1;
                    }
                    _ => {
                        match input_map.get_mut(&packet.packet_id()) {
                            None => {
                                //unknown connection, ignore
                            }
                            Some(s) => {
                                let cid = packet.connection_id();
                                match s.send(packet).await {
                                    Ok(_) => {}
                                    Err(_) => {
                                        eprintln!("Packet for dead connection handler discarded!");
                                        input_map.remove(&cid);
                                        output_map.remove(&cid);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });

        Ok(())
    }
}
