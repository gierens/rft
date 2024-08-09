use crate::loss_simulation::LossSimulation;
use std::{net::IpAddr, path::PathBuf};

pub struct Client {
    loss_sim: Option<LossSimulation>,
}

impl Client {
    pub fn new(loss_sim: Option<LossSimulation>) -> Self {
        Client { loss_sim }
    }

    pub fn run(&self, host: IpAddr, port: u16, files: Vec<PathBuf>) {
        println!("Client querying server on {}:{}", host, port);
        println!("Files to download: {:?}", files);
    }
}
