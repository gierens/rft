use crate::stream_handler::stream_handler;
use crate::wire::{ErrorFrame, Frame, Packet, Size};
use futures::{Sink, SinkExt, Stream, StreamExt};
use std::cmp::min;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::{Arc, Condvar, Mutex};

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
    let flowwnd = Arc::new(Mutex::new(2048u32));
    let last_ackd_ids: Arc<(Mutex<[u32; 2]>, Condvar)> =
        Arc::new((Mutex::new([0, 0]), Condvar::new()));

    //slow start threshold
    let mut cwnd = Arc::new(Mutex::new((4u32, u32::MAX, false)));
    //let mut ssthresh:u32 = u32::MAX;
    //let mut congestion_avoidance = false;

    //create mpsc channel for multiplexing  TODO: what is a good buffer size here?
    let (mux_tx, mut mux_rx) = futures::channel::mpsc::channel(32);

    //TODO: maybe avoid 'static somehow?

    //start frame switch task
    let connid_switch = connection_id.clone();
    let flowwnd_switch = flowwnd.clone();
    let cwnd_switch = cwnd.clone();
    let last_ackids_switch = last_ackd_ids.clone();
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
                        match frame {
                            Frame::Exit(_) => {
                                //TODO: how to kill all the handler processes? -> likely best solution: just let them time out
                                //handlers will terminate if input channels are closed //TODO: read
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
                            Frame::FlowControl(f) => {
                                //update flow window size
                                let mut fwnd_mtx = flowwnd_switch.lock().unwrap();
                                *fwnd_mtx = f.window_size();
                            }
                            Frame::Ack(f) => {
                                let (lock, cvar) = &*last_ackids_switch;
                                let id0;
                                let id1;

                                {
                                    //update last ACKd packet ID
                                    let mut ids = lock.lock().unwrap();
                                    ids[1] = ids[0];
                                    ids[0] = f.packet_id();

                                    id0 = ids[0];
                                    id1 = ids[1];
                                }

                                //update congestion window
                                let mut cwnd_mtx = cwnd_switch.lock().unwrap();
                                if cwnd_mtx.2 {
                                    if id0 > id1 {
                                        cwnd_mtx.0 += (1024 * (id0 - id1)) / cwnd_mtx.0;
                                    } else {
                                        cwnd_mtx.0 /= 2;
                                    }
                                } else if id0 > id1 {
                                    cwnd_mtx.0 += 1024 * (id0 - id1);
                                } else {
                                    //TCP Reno
                                    cwnd_mtx.0 /= 2;
                                    cwnd_mtx.1 = cwnd_mtx.0;
                                    cwnd_mtx.2 = true;
                                }

                                if cwnd_mtx.0 >= cwnd_mtx.1 {
                                    cwnd_mtx.2 = true;
                                }

                                //wake up packet assembler waiting for ACK
                                cvar.notify_one();
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
    let mut packet_id = 0; //last used packet ID, increment before use
    let mut tx_packet_id = 0; // next packet id to be sent - 1 (for rewinding)
    let mut last_ackd_pckt_id = 0;
    let total_bytes = 0u64;
    let last_ackd_bytes = 0u64;

    let ringbuf_size = 2048; //this is fixed, has to be large enough

    //ring buffer for sizes of sent packets
    let mut ringbuf_szs: Vec<u32> = Vec::new();
    let mut ringbuf_szs_head = 0;
    ringbuf_szs.resize(ringbuf_size, 0);

    //ring buffer for sent packets
    let mut ringbuf_pkts: Vec<Packet> = Vec::new();
    let mut ringbuf_pkts_head = 0; //last written element (increment before write)
    ringbuf_pkts.resize(ringbuf_size, Packet::new(0, 0));

    let mut peeked_frame: Vec<Frame> = Vec::new();
    let max_packet_size = 1024;

    loop {
        //get connection id
        let connid;
        {
            connid = *connection_id.lock().unwrap();
        }
        let mut packet = Packet::new(connid, packet_id + 1);

        //check if we need to wait for ACK, rewind, or continue TODO: timeout and re-slow start
        let flowwnd_sample;
        let cwnd_sample;
        {
            flowwnd_sample = *flowwnd.lock().unwrap();
        }
        {
            cwnd_sample = *cwnd.lock().unwrap();
        }
        if tx_packet_id - last_ackd_pckt_id >= min(flowwnd_sample, cwnd_sample.0) {
            let mut illegal_ack = false;

            {
                let (lock, cvar) = &*last_ackd_ids;
                let mut ids = lock.lock().unwrap();
                loop {
                    if ids[0] > last_ackd_pckt_id {
                        //new ACK received
                        last_ackd_pckt_id = ids[0];
                        break;
                    }
                    if ids[0] == last_ackd_pckt_id && ids[0] > ids[1] {
                        //no new ACK received, wait and continue
                        ids = cvar.wait(ids).unwrap();
                        continue;
                    }
                    if ids[0] == ids[1] {
                        //double ACK received, rewind
                        tx_packet_id = last_ackd_pckt_id;
                        break;
                    }

                    //else: should never get here
                    illegal_ack = true;
                    break;
                }
            }

            if illegal_ack {
                packet.add_frame(
                    ErrorFrame::new(0, "ACK irregularities observed, terminating connection")
                        .into(),
                );
                sink.send(packet).await.expect("could not send packet");
                return Ok(());
            }
        }

        if packet_id == tx_packet_id {
            //get some frames and add them to packet
            let mut size = 0;
            loop {
                //TODO: how long to wait for more frames?
                let frame = if !peeked_frame.is_empty() {
                    peeked_frame.pop().unwrap()
                } else {
                    match mux_rx.next().await {
                        None => return Ok(()),
                        Some(f) => f,
                    }
                };

                //continue only if frame is data frame with payload size > 0 (not EOF)
                let send_pckt: bool = match frame {
                    Frame::Data(_) => frame.size() > 74,
                    _ => true,
                };

                //check if max size surpassed -> save overhanging frame and break
                if size + frame.size() > max_packet_size {
                    peeked_frame.push(frame);
                    break;
                }

                size += packet.size(); //TODO how to measure actual size?
                packet.add_frame(frame);

                //break for frames to be send immediately
                if send_pckt {
                    break;
                }
            }

            //insert packet size to packet size ring buffer
            ringbuf_szs_head += (ringbuf_szs_head + 1) % ringbuf_size;
            ringbuf_szs[ringbuf_szs_head] = packet.size() as u32;

            //insert packet to ring buffer
            ringbuf_pkts_head = (ringbuf_pkts_head + 1) % ringbuf_size;
            ringbuf_pkts[ringbuf_pkts_head] = packet.clone();
            //TODO: delete packets out of window to save memory
        } else {
            //resend from ring buffer
            packet = ringbuf_pkts[((tx_packet_id + 1) as usize) % ringbuf_size].clone();
        }

        //send packet trough sink
        sink.send(packet).await.expect("could not send packet");

        //if rewinding, increment only tx_packet_id
        if packet_id == tx_packet_id {
            packet_id += 1;
        }
        tx_packet_id += 1;
    }
}
