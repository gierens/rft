use crate::loss_simulation::LossSimulation;
use crate::wire::*;
use crate::stream_handler::stream_handler;
use futures::channel::mpsc::{channel, Receiver, Sender};
use anyhow::{anyhow, Context};
use futures::SinkExt;
use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};
use std::path::PathBuf;

#[derive(Debug)]
pub struct ClientConfig {
    pub host: Ipv4Addr,
    pub port: u16,
    pub files: Vec<PathBuf>,
    pub loss_sim: Option<LossSimulation>,
}

impl ClientConfig {
    pub fn new(
        host: Ipv4Addr,
        port: u16,
        files: Vec<PathBuf>,
        loss_sim: Option<LossSimulation>,
    ) -> Self {
        Self {
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
    sinks: Vec<Sender<Frame>>,
    handles: Vec<tokio::task::JoinHandle<Result<(), anyhow::Error>>>,
}

impl Client {
    pub fn new(config: ClientConfig) -> Self {
        Client {
            config,
            conn: None,
            sinks: Vec::new(),
            handles: Vec::new(),
        }
    }

    /// Connect the client to the specified server
    pub fn connect(&mut self) -> Result<&Client, anyhow::Error> {
        let socket = match UdpSocket::bind("0.0.0.0") {
            Ok(socket) => {
                match socket.connect(SocketAddrV4::new(self.config.host, self.config.port)) {
                    Ok(_) => socket,
                    Err(e) => return Err(anyhow!("Failed to connect to server: {}", e)),
                }
            }
            Err(e) => return Err(anyhow!("Failed to bind socket: {}", e)),
        };
        self.conn = Some(socket);
        Ok(self)
    }

   
    pub async fn start(&mut self) -> Result<(), anyhow::Error> {
        // idea: https://excalidraw.com/#json=SmceuVrZR7teBVxFnKskC,6anX_11ILOMBKLWYSJQrng
        let conn = self.conn.as_ref().context("Connection not established")?;

        // TODO Start connection establishment and ConnID
        let packet = Packet::new(0);
        let bytes = packet.assemble();
        conn.send(&bytes).context("Failed to send packet")?;
        
        let mut buf = [0; 1024];
        let size = conn.recv(&mut buf)?;
        let packet = Packet::parse_buf(&buf[..size]).context("Failed to parse packet")?;
        
        // Check for ConnID
        let conn_id = packet.header().connection_id;
        if conn_id == 0 {
            return Err(anyhow!("Failed to establish connection"));
        };

        // send frames on one stream per file
        // one stream handler per file
        // send server read cmd first
        // we need one receiver in total, and one sender per file/command, create via cloning
        //let out_sink = out_tx.clone();
        //tokio::spawn(async move {
        //    stream_handler(in_rx, out_sink).await;
        //});

        let (server_out_tx, server_out_rx): (Sender<Frame>, Receiver<Frame>) = channel(3);
        let (server_in_tx, server_in_rx): (Sender<Frame>, Receiver<Frame>) = channel(3);
        
        // Create a sink (sender) for each file and have the same receiver (server)
        let mut sinks: Vec<Sender<Frame>> = Vec::new();
        for _ in &self.config.files {
            let sink = server_in_tx.clone();
            sinks.push(sink);
        }

        // Start the stream_handlers
        for sink in &mut self.sinks {
            let handle = tokio::spawn(stream_handler(server_in_rx, sink));
            self.handles.push(handle);
        }

        // Send the read command to the server
        for (sink, file) in sinks.iter().zip(&self.config.files) {
            let path = file.as_path();
            let read_frame = ReadFrame::new(0, 0, 0, 0, 1024, 0, path);
            let read_cmd = Frame::Read(read_frame);
            sink.clone().send(read_cmd).await.expect("Failed to send read command");
        }
    
        // Start assembling frames to packets and send them to the server
        loop {
            let mut packet = Packet::new(conn_id);
            let mut size = 0;
            loop {
                // TODO: find proper value or make it depend on the packet content
                if size > 2 {
                    break;
                }
                for sink in &mut self.sinks {
                    let frame = match sink.next().await {
                        Some(frame) => frame,
                        None => continue,
                    };
                    packet.add_frame(frame);
                    size += 1;
                }
            }
            
            server_out_rx.send(packet).await.expect("Failed to send packet");
        }

        Ok(())
    }

    // sink switcher

//    /// Create files for writing the requested files
//    fn create_files(&mut self) -> io::Result<()> {
//        let mut files = Vec::new();
//
//        for f_path in &self.config.files {
//            let file = OpenOptions::new()
//                .read(true)
//                .write(true)
//                .create(true)
//                .truncate(true)
//                .open(f_path)?;
//            
//            files.push(file);
//        }
//
//        self.files = files;
//        Ok(())
//    }
}
