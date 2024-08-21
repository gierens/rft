use crate::loss_simulation::LossSimulation;
use crate::stream_handler::stream_handler;
use crate::wire::*;
use anyhow::{anyhow, Context};
use futures::channel::mpsc::{channel, Receiver, Sender};
use futures::{SinkExt, StreamExt};
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

    // TODO: check for good buffer sizes
    #[tokio::main]
    pub async fn start(&mut self) -> Result<(), anyhow::Error> {
        // idea: https://excalidraw.com/#json=SmceuVrZR7teBVxFnKskC,6anX_11ILOMBKLWYSJQrng
        // send frames on one stream per file
        // one stream handler per file
        // send server read cmd first
        // we need one receiver in total, and one sender per file/command, create via cloning
        //let out_sink = out_tx.clone();
        //tokio::spawn(async move {
        //    stream_handler(in_rx, out_sink).await;
        //});

        let conn = self.conn.as_ref().context("Connection not established")?;
        let packet_id = 0;

        // TODO Start connection establishment and ConnID
        let packet = Packet::new(0, packet_id);
        let bytes = packet.assemble();
        conn.send(&bytes).context("Failed to send packet")?;

        let mut buf = [0; 1024];
        let size = conn.recv(&mut buf)?;
        let packet = Packet::parse_buf(&buf[..size]).context("Failed to parse packet")?;

        // Check for ConnID
        let conn_id = packet.header().connection_id;
        if conn_id == 0 {
            return Err(anyhow!("Failed to establish connection, received ConnID 0"));
        };

        let (server_out_tx, mut server_out_rx): (Sender<Frame>, Receiver<Frame>) = channel(3);
        let (server_in_tx, server_in_rx): (Sender<Frame>, Receiver<Frame>) = channel(3);

        // Create a sink (sender) for each file and have the same receiver (server)
        let mut sinks: Vec<Sender<Frame>> = Vec::new();
        for _ in &self.config.files {
            let sink = server_in_tx.clone();
            sinks.push(sink);
        }

        // Start the stream_handlers
        for sink in &self.sinks {
            let handle = tokio::spawn(stream_handler(server_in_rx, sink.clone())); // TODO cloning here okay?
            self.handles.push(handle);
        }

        // Send the read commands to the server
        for (sink, file) in sinks.iter().zip(&self.config.files) {
            let path = file.as_path();
            let read_frame = ReadFrame::new(0, 0, 0, 1024, 0, path);
            let read_cmd = Frame::Read(read_frame);
            sink.clone()
                .send(read_cmd)
                .await
                .expect("Failed to send read command");
        }

        // Start the packet switcher
        // Run as async task
        // Switch for WriteFrame based on path, for the others idk
        // TODO
        // tokio::spawn(Self::packet_switcher(server_in_rx, sinks));

        // Start assembling frames to packets and send them to the server
        tokio::spawn(Self::packet_assembly_and_sender(
            server_out_rx,
            conn.try_clone()?,
            conn_id,
        ));

        return Ok(());
    }

    /// Switches the incoming Packets from the server and distributes the Frames in the Packets to the responsible sinks
    /// to allow the correct stream_handler to handle the Frames
    async fn packet_switcher(
        server_in_rx: Receiver<Packet>, // TOOD: do we receive packets over the UDPSocket or channel?
        sinks: Vec<Sender<Frame>>,
    ) -> Result<(), anyhow::Error> {
        // TODO
        Ok(())
    }

    async fn packet_assembly_and_sender(
        mut server_out_rx: Receiver<Frame>,
        socket: UdpSocket,
        conn_id: u32,
    ) -> Result<(), anyhow::Error> {
        // TODO: proper usage of packet_id
        let mut packet_id = 0;

        while let Some(frame) = server_out_rx.next().await {
            let mut packet = Packet::new(conn_id, packet_id);
            packet.add_frame(frame);

            for _ in 0..2 {
                // Add up to 2 more frames per packet
                if let Some(frame) = server_out_rx.next().await {
                    packet.add_frame(frame);
                } else {
                    break;
                }
            }

            let buf = packet.assemble();
            socket.send(&buf).context("Failed to send packet")?;
            packet_id += 1;
        }

        Ok(())
    }

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
