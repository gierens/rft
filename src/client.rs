use crate::loss_simulation::LossSimulation;
use anyhow::anyhow;
use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};
use std::path::PathBuf;

#[derive(Debug)]
pub struct ClientConfig {
    host: Ipv4Addr,
    port: u16,
    #[allow(dead_code)]
    files: Vec<PathBuf>,
    #[allow(dead_code)]
    loss_sim: Option<LossSimulation>,
}

impl ClientConfig {
    pub fn new(
        host: Ipv4Addr,
        port: u16,
        files: Vec<PathBuf>,
        loss_sim: Option<LossSimulation>,
    ) -> Self {
        ClientConfig {
            host,
            port,
            files,
            loss_sim,
        }
    }
}

#[derive(Debug)]
pub struct Client {
    config: ClientConfig,
    conn: Option<UdpSocket>,
}

impl Client {
    pub fn new(config: ClientConfig) -> Self {
        Client { config, conn: None }
    }

    pub fn start(&mut self) -> Result<(), anyhow::Error> {
        let socket = match UdpSocket::bind("0.0.0.0.0") {
            Ok(socket) => {
                match socket.connect(SocketAddrV4::new(self.config.host, self.config.port)) {
                    Ok(_) => socket,
                    Err(e) => return Err(anyhow!("Failed to connect to server: {}", e)),
                }
            }
            Err(e) => return Err(anyhow!("Failed to bind socket: {}", e)),
        };
        self.conn = Some(socket);
        Ok(())
    }
}
