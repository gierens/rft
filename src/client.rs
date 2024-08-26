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
    #[allow(dead_code)]
    pub loss_sim: Option<LossSimulation>,
}

impl ClientConfig {
    pub fn new(
        host: Ipv4Addr,
        port: u16,
        files: Vec<PathBuf>,
        #[allow(dead_code)] loss_sim: Option<LossSimulation>,
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
}

impl Client {
    pub fn new(config: ClientConfig) -> Self {
        Client {
            config,
            conn: None,
            sinks: Vec::new(),
        }
    }

    /// Connect the client to the specified server
    pub fn connect(&mut self) -> Result<&Client, anyhow::Error> {
        let socket = match UdpSocket::bind("0.0.0.0:0") {
            Ok(socket) => {
                match socket.connect(SocketAddrV4::new(self.config.host, self.config.port)) {
                    Ok(_) => socket,
                    Err(e) => return Err(anyhow!("Failed to connect to server: {}", e)),
                }
            }
            Err(e) => return Err(anyhow!("Failed to bind socket: {}", e)),
        };
        self.conn = Some(socket);
        println!("DEBUG: Connected to server at {}:{}", self.config.host, self.config.port);
        Ok(self)
    }

    // TODO: check buffer sizes
    // TODO: handle congestion control
    #[tokio::main]
    pub async fn start(&mut self) -> Result<(), anyhow::Error> {
        // idea: https://excalidraw.com/#json=tbYyeXwmjsAWzIbHJqoa2,lxc2VI0v4LzKGLqVhFwotw
        // send frames on one stream per file
        // one stream handler per file

        let conn = self.conn.as_ref().context("Connection not established")?;
        let mut packet_id = 1; // client counter for the packet_id
        let mut last_recv_packet_id;
        let mut recv_buf: [u8; 1024] = [0; 1024];

        // Start connection establishment and ConnID
        // TODO: handle connection establishment with CID change Frame
        let packet = Packet::new(0, packet_id);
        let bytes = packet.assemble();
        conn.send(&bytes).context("Failed to send packet")?;

        let size = conn.recv(&mut recv_buf)?;
        let packet = Packet::parse_buf(&recv_buf[..size]).context("Failed to parse packet")?;

        // Check for connection establishment
        let conn_id = packet.header().connection_id;
        if conn_id == 0 {
            return Err(anyhow!("Failed to establish connection, received ConnID 0"));
        };
        last_recv_packet_id = packet.header().packet_id;
        if last_recv_packet_id != 1 {
            println!("WARN: received unexpected packet_id from the server during connection establishment: {}", last_recv_packet_id);
        }

        let mut transmission_complete = vec![false; self.config.files.len()];

        let (assembler_sink, mut assembler_rx): (Sender<Frame>, Receiver<Frame>) = channel(3);

        // Setup up channels for stream handlers and assembler
        for _ in &self.config.files {
            let (tx, rx): (Sender<Frame>, Receiver<Frame>) = channel(3);
            self.sinks.push(tx);
            let assembly_sink = assembler_sink.clone();

            // Start the stream handlers
            tokio::spawn(stream_handler(rx, assembly_sink));
        }

        // Start the packet assembler and sender
        let conn_clone = conn.try_clone().context("Failed to clone connection")?;
        tokio::spawn(async move {
            while let Some(frame) = assembler_rx.next().await {
                let mut packet = Packet::new(conn_id, packet_id);
                packet.add_frame(frame);
                for _ in 0..2 {
                    // Add up to 2 more frames per packet
                    if let Some(frame) = assembler_rx.next().await {
                        packet.add_frame(frame);
                    } else {
                        break;
                    }
                }
                let buf = packet.assemble();
                conn_clone
                    .send(&buf)
                    .context("Failed to send packet")
                    .unwrap();
                packet_id += 1;
            }
        });

        // Send WriteFrame's to ourselves to create the requested files
        for (i, path) in self.config.files.iter().enumerate() {
            let write_frame = WriteFrame::new((i + 1) as u16, 0, 0, path);
            self.sinks[i].send(Frame::Write(write_frame)).await?;
        }

        // Send the ReadFrame's to the server to read the entire files
        for (i, path) in self.config.files.iter().enumerate() {
            let read_frame = ReadFrame::new((i + 1) as u16, 0, 0, 0, 0, path);
            let mut packet = Packet::new(conn_id, packet_id);
            packet.add_frame(Frame::Read(read_frame));
            let bytes = packet.assemble();
            conn.send(&bytes).context("Failed to send packet")?;
        }

        // Receive the Packets from the server and switch the contained Frames to the corresponding sinks
        while transmission_complete.iter().all(|&x| x) {
            let size = conn.recv(&mut recv_buf)?;
            let packet = Packet::parse_buf(&recv_buf[..size])?;
            let _recv_packet_id = packet.header().packet_id;
            if _recv_packet_id != last_recv_packet_id + 1 {
                println!(
                    "WARN: received unexpected packet_id from the server, expected {} but got {}",
                    last_recv_packet_id + 1,
                    _recv_packet_id
                );
            }
            last_recv_packet_id = _recv_packet_id;

            let frames = packet.frames;
            for frame in frames {
                let stream_id = frame.stream_id();
                if stream_id == 0 {
                    // TODO: handle control frames
                    println!(
                        "Received control frame. Is this important? ¯\\_(ツ)_/¯: {:?}",
                        frame
                    );
                    continue;
                }

                let n = stream_id as usize;
                if n - 1 > self.sinks.len() {
                    println!(
                        "WARN: received frame for unknown stream with stream_id: {}. Ignoring it.",
                        n
                    );
                    continue;
                }

                // Check if it is the last data frame
                if let Frame::Data(data_frame) = &frame {
                    if data_frame.length() == 0 {
                        transmission_complete[n - 1] = true;
                    }
                }

                // Send frame to corresponding sink
                self.sinks[n - 1].send(frame).await?;
            }
        }

        // Send Exit Frame
        let mut packet = Packet::new(conn_id, packet_id);
        packet.add_frame(Frame::Exit(ExitFrame::new()));
        let bytes = packet.assemble();
        conn.send(&bytes).context("Failed to send packet")?;

        return Ok(());
    }
}
