use crate::stream_handler::stream_handler;
use crate::loss_simulation::LossSimulation;
use crate::wire::{Frame, Packet};
use futures::SinkExt;
use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};

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
        let socket = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), self.port))
            .expect("Failed to bind socket");
        let mut buf = [0; 1024];

        //create mpsc channel for multiplexing  TODO: what is a good buffer size here?
        let (mux_tx, _mux_rx) = futures::channel::mpsc::channel(32);

        //start frame switch task
        tokio::spawn(async move {
            //hash map for handler input channels
            let mut handler_map: HashMap<u16, futures::channel::mpsc::Sender<Frame>> =
                HashMap::new();

            loop {
                let size = match socket.recv(&mut buf) {
                    Ok(size) => size,
                    Err(e) => {
                        eprintln!("Failed to receive data: {}", e);
                        continue;
                    }
                };
                let packet = Packet::parse_buf(&buf[..size]).expect("Failed to parse packet");
                //dbg!(packet);

                for frame in packet.frames {
                    match frame.stream_id() {
                        0 => {
                            //TODO: handle connection control frames
                        }
                        _ => {
                            match handler_map.get_mut(&frame.stream_id()) {
                                None => {
                                    //create new channel
                                    let (mut ctx, crx) = futures::channel::mpsc::channel(8); //TODO: good buffer size?

                                    //send frame
                                    let sid = frame.stream_id();
                                    ctx.send(frame).await.unwrap();

                                    //add sink to hashmap
                                    handler_map.insert(sid, ctx);

                                    //start new handler
                                    let mux_tx_c = mux_tx.clone();
                                    tokio::spawn(async move {
                                        stream_handler(crx, mux_tx_c).await.expect("handler error");
                                    });
                                }
                                Some(s) => {
                                    //try to send to sink
                                    match s.try_send(frame) {
                                        Ok(_) => {
                                            //OK, handler alive
                                        }
                                        Err(e) => {
                                            //check if reason for error was handler being dead
                                            if !e.is_disconnected() {
                                                eprintln!("Handler input error: {}", e);
                                            }

                                            //handler dead, start new one
                                            //create new channel
                                            let (mut ctx, crx) =
                                                futures::channel::mpsc::channel(16); //TODO: good buffer size?

                                            //send frame
                                            let f = e.into_inner();
                                            let sid = f.stream_id();
                                            ctx.send(f)
                                                .await
                                                .expect("error sending to new channel (???)");

                                            //add sink to hashmap
                                            handler_map.insert(sid, ctx);

                                            //start new handler
                                            let mux_tx_c = mux_tx.clone();
                                            tokio::spawn(async move {
                                                stream_handler(crx, mux_tx_c.clone())
                                                    .await
                                                    .expect("handler error");
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });

        Ok(())

        //do frame muxing
        /*
        loop {
            //take frames from mpsc stream and assemble+send packets
        }
         */
    }
}
