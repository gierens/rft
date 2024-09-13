use crate::loss_simulation::LossSimulation;
use crate::stream_handler::stream_handler;
use crate::wire::*;
use anyhow::{anyhow, Context};
use futures::channel::mpsc::{channel, Receiver, Sender};
use futures::{SinkExt, StreamExt};
use log::{debug, error, info, warn};
use std::net::{Ipv4Addr, SocketAddrV4};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::task::spawn_blocking;
use tokio::time::sleep;

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
    sinks: Vec<Sender<Frame>>,
}

impl Client {
    pub fn new(config: ClientConfig) -> Self {
        Client {
            config,
            sinks: Vec::new(),
        }
    }

    pub async fn start(&mut self) -> Result<(), anyhow::Error> {
        // Connect the client to the specified server
        let socket = match UdpSocket::bind("0.0.0.0:0").await {
            Ok(socket) => {
                match socket
                    .connect(SocketAddrV4::new(self.config.host, self.config.port))
                    .await
                {
                    Ok(_) => socket,
                    Err(e) => return Err(anyhow!("Failed to connect to server: {}", e)),
                }
            }
            Err(e) => return Err(anyhow!("Failed to bind socket: {}", e)),
        };
        let conn = Arc::new(socket);
        info! {"Connected to server at {}:{}", self.config.host, self.config.port};

        // TODO: check buffer sizes
        // TODO: handle congestion control
        // idea: https://excalidraw.com/#json=tbYyeXwmjsAWzIbHJqoa2,lxc2VI0v4LzKGLqVhFwotw
        // send frames on one stream per file
        // one stream handler per file

        let mut packet_id = 1; // client counter for the packet_id
        let mut last_recv_packet_id;
        let mut recv_buf: [u8; 2048] = [0; 2048];

        // Start connection establishment and ConnID
        // TODO: handle connection establishment with CID change Frame
        let packet = Packet::new(0, packet_id);
        let bytes = packet.assemble();
        conn.send(&bytes).await.context("Failed to send packet")?;
        packet_id += 1;

        let size = conn.recv(&mut recv_buf).await?;
        let packet = Packet::parse_buf(&recv_buf[..size]).context("Failed to parse packet")?;

        // Check for connection establishment
        let conn_id = packet.header().connection_id;
        if conn_id == 0 {
            return Err(anyhow!("Failed to establish connection, received ConnID 0"));
        };
        last_recv_packet_id = packet.header().packet_id;
        if last_recv_packet_id != 1 {
            warn!(
                "Received unexpected packet_id from the server during connection establishment: {}",
                last_recv_packet_id
            );
        }

        let mut transmission_complete = vec![false; self.config.files.len()];

        let (mut assembler_sink, mut assembler_rx): (Sender<Frame>, Receiver<Frame>) = channel(3);

        debug! {"Starting {} stream handlers", self.config.files.len()};

        // Setup up channels for stream handlers and assembler
        for _ in &self.config.files {
            let (tx, rx): (Sender<Frame>, Receiver<Frame>) = channel(3);
            self.sinks.push(tx);
            let assembly_sink = assembler_sink.clone();

            // Start the stream handlers
            tokio::spawn(stream_handler(rx, assembly_sink));
        }

        // Start the packet assembler and sender
        let conn_clone = conn.clone();
        tokio::spawn(async move {
            while let Some(frame) = assembler_rx.next().await {
                let mut packet = Packet::new(conn_id, packet_id);

                sleep(Duration::from_millis(5)).await;
                match frame {
                    Frame::Ack(mut ack_frame) => {
                        for _ in 0..10 {
                            // info!("trying to reduce ack spam, take {}...", i);
                            if let Ok(Some(frame2)) = assembler_rx.try_next() {
                                match frame2 {
                                    Frame::Ack(ack_frame2) => {
                                        ack_frame = ack_frame2;
                                    }
                                    _ => {
                                        packet.add_frame(ack_frame.into());
                                        packet.add_frame(frame2);
                                        break;
                                    }
                                }
                            } else {
                                packet.add_frame(ack_frame.into());
                                break;
                            }
                        }
                    }
                    _ => {
                        packet.add_frame(frame);
                    }
                }

                debug!("Sending packet with packet {:?}", &packet);
                let buf = spawn_blocking(move || packet.assemble()).await.unwrap();
                conn_clone
                    .send(&buf)
                    .await
                    .context("Failed to send packet")
                    .unwrap();
                packet_id += 1;
            }
        });

        debug! {"Sending {} WriteFrames to create files", self.config.files.len()};
        // Send WriteFrame's to ourselves to create the requested files
        for (i, path) in self.config.files.iter().enumerate() {
            let write_frame = WriteFrame::new((i + 1) as u16, 0, 0, path);
            self.sinks[i].send(Frame::Write(write_frame)).await?;
            debug!("Sent WriteFrame for file: {:?} to sink {}", path, i);
        }

        debug! {"Sending {} ReadFrames to server to read files", self.config.files.len()};
        // Send the ReadFrame's to the server to read the entire files
        for (i, path) in self.config.files.iter().enumerate() {
            assembler_sink
                .send(Frame::Read(ReadFrame::new(
                    (i + 1) as u16,
                    0,
                    0,
                    0,
                    0,
                    path,
                )))
                .await?;
        }

        // Receive the Packets from the server and switch the contained Frames to the corresponding sinks
        while !transmission_complete.iter().all(|&x| x) {
            // TODO send ack on timeout of a few ms maybe
            let size = conn.recv(&mut recv_buf).await?;
            let packet = Packet::parse_buf(&recv_buf[..size])?;
            let _recv_packet_id = packet.header().packet_id;
            if _recv_packet_id != last_recv_packet_id + 1 {
                warn!(
                    "Received unexpected packet_id from the server, expected {} but got {}",
                    last_recv_packet_id + 1,
                    _recv_packet_id
                );
            }
            last_recv_packet_id = _recv_packet_id;
            assembler_sink
                .send(Frame::Ack(AckFrame::new(last_recv_packet_id)))
                .await?;

            let frames = packet.frames;
            for frame in frames {
                let stream_id = frame.stream_id();
                if stream_id == 0 {
                    // TODO: handle control frames
                    error!(
                        "Received unhandled control frame. Is this important? ¯\\_(ツ)_/¯: {:?}",
                        frame
                    );
                    continue;
                }

                let n = stream_id as usize;
                if n - 1 > self.sinks.len() {
                    warn!(
                        "Received frame for unknown stream with stream_id: {}. Ignoring it.",
                        n
                    );
                    continue;
                }

                // Check if it is the last data frame
                if let Frame::Data(data_frame) = &frame {
                    if data_frame.length() == 0 {
                        info!("Received last data for stream {}: {:?}", n - 1, data_frame);
                        info!("Transmission complete for stream {}", n - 1);
                        transmission_complete[n - 1] = true;
                    }
                }

                // Send frame to corresponding sink
                self.sinks[n - 1].send(frame).await?;
                debug!("Sent frame to sink {}", n - 1);
            }
        }

        debug!("Transmission complete. Closing connection...");
        // Send Exit Frame
        let mut packet = Packet::new(conn_id, packet_id);
        packet.add_frame(Frame::Exit(ExitFrame::new()));
        let bytes = spawn_blocking(move || packet.assemble()).await?;
        conn.send(&bytes).await.context("Failed to send packet")?;
        debug!("Sent ExitFrame to server with packet_id {}", packet_id);
        Ok(())
    }
}
