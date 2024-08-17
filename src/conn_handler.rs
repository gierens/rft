use crate::stream_handler::stream_handler;
use crate::wire::{Frame, Packet};
use futures::{Sink, SinkExt, Stream, StreamExt};
use std::collections::HashMap;
use std::fmt::Debug;
use anyhow::anyhow;

pub async fn connection_handler<S: Sink<Packet> + Unpin>(
    mut stream: impl Stream<Item = Packet> + Unpin + Send + 'static,
    mut sink: S,
) -> anyhow::Result<()>
where
<S as futures::Sink<Packet>>::Error: Debug,
{
    //create mpsc channel for multiplexing  TODO: what is a good buffer size here?
    let (mux_tx, _mux_rx) = futures::channel::mpsc::channel(32);

    //TODO: maybe avoid 'static somehow?

    //start frame switch task
    tokio::spawn(async move {
        //hash map for handler input channels
        let mut handler_map: HashMap<u16, futures::channel::mpsc::Sender<Frame>> =
            HashMap::new();

        loop {

            let packet = match stream.next().await {
                None => {return anyhow!("packet stream closed!");}
                Some(p) => {p}
            };


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

    //do frame muxing
    /*
    loop {
        //take frames from mpsc stream and assemble+send packets
    }
     */

    Ok(())
}