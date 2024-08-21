use crate::stream_handler::stream_handler;
use crate::wire::{Frame, Packet};
use futures::{Sink, SinkExt, Stream, StreamExt};
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};

#[allow(dead_code)]
#[allow(unused_mut)]
#[allow(unused_variables)]
pub async fn connection_handler<S: Sink<Packet> + Unpin>(
    mut stream: impl Stream<Item = Packet> + Unpin + Send + 'static,
    mut sink: S,
) -> anyhow::Result<()>
where
    <S as futures::Sink<Packet>>::Error: Debug,
{
    //for now, assume established connection
    let connection_id = Arc::new(Mutex::new(42069u32));

    //create mpsc channel for multiplexing  TODO: what is a good buffer size here?
    let (mux_tx, mut mux_rx) = futures::channel::mpsc::channel(32);

    //TODO: maybe avoid 'static somehow?

    //start frame switch task
    let connid_switch = connection_id.clone();
    tokio::spawn(async move {
        //hash map for handler input channels
        let mut handler_map: HashMap<u16, futures::channel::mpsc::Sender<Frame>> = HashMap::new();

        loop {
            let packet = match stream.next().await {
                None => {
                    return;
                }
                Some(p) => p,
            };

            for frame in packet.frames {
                match frame.stream_id() {
                    0 => {
                        //TODO: handle connection control frames
                        match frame {
                            Frame::Exit(_) => {
                                //TODO: how to kill all the handler processes? -> likely best solution: just let them time out
                                //parent process will return if mpsc channel has no more senders
                                return;
                            }
                            Frame::ConnIdChange(f) => {
                                //TODO: have mutex'd connId variable and change it here
                                //check old stream ID
                                {
                                    if *connid_switch.lock().unwrap() != f.old_cid() {
                                        //for now: ignore
                                        //TODO: ???
                                        eprintln!("Wrong old CID in connection handler change_CID");
                                    }
                                }

                                //update CID
                                let mut cid_mtx = connid_switch.lock().unwrap();
                                *cid_mtx = f.new_cid();
                            }
                            Frame::FlowControl(_) => {
                                //TODO
                            }
                            _ => {}
                        }
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
                                        let (mut ctx, crx) = futures::channel::mpsc::channel(16); //TODO: good buffer size?

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

    //start frame muxing and packet assembly
    let mut packet_id = 0;
    loop {
        //get connection id
        let connid;
        {
            connid = *connection_id.lock().unwrap();
        }
        let mut packet = Packet::new(connid, packet_id);

        //get some frames and add them to packet
        let mut size = 0;
        loop {
            if size > 5 {
                break;
            }
            //TODO: how long to wait for more frames?
            let frame = match mux_rx.next().await {
                None => return Ok(()),
                Some(f) => f,
            };

            size += 1; //TODO how to measure actual size?
            packet.add_frame(frame);
        }

        //send packet trough sink
        sink.send(packet).await.expect("could not send packet");
        packet_id += 1;
    }
}
