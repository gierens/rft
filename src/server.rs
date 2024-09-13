use crate::conn_handler::connection_handler;
use crate::loss_simulation::LossSimulation;
use crate::wire::{Assemble, Packet};
use futures::channel::mpsc;
use futures::{SinkExt, StreamExt};
use log::{debug, error, info, warn};
use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::task::spawn_blocking;
use tokio::time::timeout;

pub struct Server {
    port: u16,
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
        self::Server::print_banner();
        info!("Server running on port {}", self.port);
        //HashMap for client IPs
        //let mut output_map: HashMap<u32, SocketAddr> = HashMap::new();
        let output_map: Arc<Mutex<HashMap<u32, SocketAddr>>> = Arc::new(Mutex::new(HashMap::new()));
        let mut loss_sim = self.loss_sim.clone().map(|loss_sim| Arc::new(Mutex::new(loss_sim)));

        //HashMap for connection handlers
        let mut input_map: HashMap<u32, mpsc::Sender<Packet>> = HashMap::new();

        //mpsc channel <Packet>: handler output -> transmitter input
        let (mux_tx, mut mux_rx) = mpsc::channel(32);

        let socket = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), self.port))
            .await
            .expect("Failed to bind socket");
        let udp_rx = Arc::new(socket);
        let udp_tx = udp_rx.clone();

        //TODO: delete closed connections from HashMaps

        //start packet switching task
        let mut output_map_switch = output_map.clone();
        let mut loss_sim_switch = loss_sim.clone();
        tokio::spawn(async move {
            let mut buf = [0; 2048];
            let mut cid_ctr = 1u32;
            loop {
                let (size, client_addr) = udp_rx
                    .recv_from(&mut buf)
                    .await
                    .expect("UDP Socket rx error");
                let packet = spawn_blocking(move || Packet::parse_buf(&buf[..size]))
                    .await
                    .unwrap()
                    .expect("Failed to parse packet");
                if let Some(loss_sim) = loss_sim_switch.as_mut() {
                    if loss_sim.lock().unwrap().drop_packet() {
                        warn!(
                            "Simulated loss of received packet {} occurred!",
                            packet.packet_id()
                        );
                        continue;
                    }
                }

                debug!("Received packet: {:?}", &packet);

                match packet.connection_id() {
                    0 => {
                        debug!("New connection, ID: {}", cid_ctr);
                        let (mut ctx, crx) = mpsc::channel(128);

                        ctx.send(packet).await.unwrap();

                        input_map.insert(cid_ctr, ctx);
                        {
                            let mut omap_mtx = output_map_switch.lock().unwrap();
                            omap_mtx.insert(cid_ctr, client_addr);
                        }

                        let mux_tx_c = mux_tx.clone();
                        tokio::spawn(async move {
                            connection_handler(crx, mux_tx_c, cid_ctr)
                                .await
                                .expect("connection handler error");
                        });

                        cid_ctr += 1;
                    }
                    _ => {
                        match input_map.get_mut(&packet.connection_id()) {
                            None => {
                                warn!(
                                    "Discard Packet for unknown connection with packet_id {}",
                                    packet.packet_id()
                                );
                            }
                            Some(s) => {
                                let cid = packet.connection_id();
                                match timeout(Duration::from_millis(1), s.send(packet)).await {
                                    Ok(r) => match r {
                                        Ok(_) => {}
                                        Err(_) => {
                                            error!("Packet for dead connection handler discarded!");
                                            input_map.remove(&cid);
                                            {
                                                let mut omap_mtx =
                                                    output_map_switch.lock().unwrap();
                                                omap_mtx.remove(&cid);
                                            }
                                        }
                                    },
                                    Err(_) => {
                                        //timeout: channel full -> drop packet
                                        error!(
                                            "connection handler input channel full, packet dropped"
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });

        //start packet sending
        loop {
            let packet = mux_rx.next().await.expect("server mux_rx closed");
            if let Some(loss_sim) = loss_sim.as_mut() {
                if loss_sim.lock().unwrap().drop_packet() {
                    warn!(
                        "Simulated loss of sent packet {} occurred!",
                        packet.packet_id()
                    );
                    continue;
                }
            }
            let dest;
            {
                let omap_mtx = output_map.lock().unwrap();
                dest = *omap_mtx
                    .get(&packet.connection_id())
                    .expect("connID not in output_map at tx");
            }
            let packet_bytes = spawn_blocking(move || packet.assemble()).await?;
            udp_tx
                .send_to(&packet_bytes, dest)
                .await
                .expect("UDP Socket tx error");
        }
    }

    fn print_banner() {
        let banner = "                                      
 ███████████   ███████████ ███████████
░░███░░░░░███ ░░███░░░░░░█░█░░░███░░░█
 ░███    ░███  ░███   █ ░ ░   ░███  ░ 
 ░██████████   ░███████       ░███    
 ░███░░░░░███  ░███░░░█       ░███    
 ░███    ░███  ░███  ░        ░███    
 █████   █████ █████          █████   
░░░░░   ░░░░░ ░░░░░          ░░░░░    
                                      ";
        println!("{}", banner);
    }
}
