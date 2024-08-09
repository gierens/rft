use std::{net::IpAddr, path::PathBuf};

pub struct Client;

impl Client {
    pub fn new() -> Self {
        Client
    }

    pub fn run(&self, host: IpAddr, port: u16, files: Vec<PathBuf>) {
        println!("Client running on {}:{}", host, port);
        println!("Files to download: {:?}", files);
    }
}
