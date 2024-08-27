use crate::stream_handler::stream_handler;
use crate::wire::{AckFrame, ErrorFrame, FlowControlFrame, Frame, Packet, Size};
use futures::{Sink, SinkExt, Stream, StreamExt};
use std::cmp::min;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;
use tokio::time::timeout;

#[allow(dead_code)]
#[allow(unused_mut)]
#[allow(unused_variables)]
pub async fn connection_handler<S: Sink<Packet> + Unpin>(
    mut stream: impl Stream<Item = Packet> + Unpin + Send + 'static,
    mut sink: S,
    connection_id: u32,
) -> anyhow::Result<()>
where
    <S as futures::Sink<Packet>>::Error: Debug,
{
    //for now, assume established connection
    let flowwnd = Arc::new(Mutex::new(2048u32));
    let last_ackd_ids: Arc<(Mutex<[u32; 2]>, Condvar)> =
        Arc::new((Mutex::new([0, 0]), Condvar::new()));

    //slow start threshold
    let mut cwnd = Arc::new(Mutex::new((4u32, u32::MAX, false)));

    //create mpsc channel for multiplexing  TODO: what is a good buffer size here?
    let (mut mux_tx, mut mux_rx) = futures::channel::mpsc::channel(16);

    //send flow control frame specifying our receive buffer size
    //TODO: this does not yet make sense, since our buffer capacity is 16 packets of arbitrary size.
    mux_tx
        .send(FlowControlFrame::new(8192).into())
        .await
        .unwrap();

    //TODO: maybe avoid 'static somehow?

    //start frame switch task
    let flowwnd_switch = flowwnd.clone();
    let cwnd_switch = cwnd.clone();
    let last_ackids_switch = last_ackd_ids.clone();
    tokio::spawn(async move {
        //hash map for handler input channels
        let mut handler_map: HashMap<u16, futures::channel::mpsc::Sender<Frame>> = HashMap::new();
        let mut last_recvd_id = 0;

        loop {
            let packet = match stream.next().await {
                None => {
                    return;
                }
                Some(p) => p,
            };

            if last_recvd_id == 0 {
                last_recvd_id = packet.packet_id();
            } else if packet.packet_id() != last_recvd_id + 1 {
                //send double ACK
                mux_tx
                    .send(AckFrame::new(last_recvd_id).into())
                    .await
                    .expect("could not send ACK");
            } else {
                last_recvd_id += 1;
            }

            //send ACK TODO: cumulative ACKs
            mux_tx
                .send(AckFrame::new(packet.packet_id()).into())
                .await
                .expect("could not send ACK");

            for frame in packet.frames {
                match frame.stream_id() {
                    0 => {
                        match frame {
                            Frame::Exit(_) => {
                                //TODO: how to kill all the handler processes? -> likely best solution: just let them time out
                                //TODO: delete closed connections from server hashmaps
                                //handlers will terminate if input channels are closed //TODO: read
                                //parent process will return if mpsc channel has no more senders
                                return;
                            }
                            Frame::ConnIdChange(f) => {
                                //TODO
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
    let mut last_ackd_pckt_id = 0; //last of our packets that was ACKd
    let mut total_bytes = 0u64; //bytes send so far, aligned with tx_packet_id (NOT packet_id)
    let mut last_ackd_bytes = 0u64;

    let ringbuf_size = 2048; //this is fixed, has to be large enough

    //ring buffer for sizes of sent packets
    let mut ringbuf_szs: Vec<u32> = vec![0; ringbuf_size];

    //ring buffer for sent packets
    let mut ringbuf_pkts: Vec<Packet> = Vec::new();
    ringbuf_pkts.resize(ringbuf_size, Packet::new(0, 0));

    let mut peeked_frame: Vec<Frame> = Vec::new();
    let max_packet_size = 1024;

    loop {
        let mut packet = Packet::new(connection_id, packet_id + 1);

        //check if we need to wait for ACK, rewind, or continue TODO: timeout and re-slow start
        let flowwnd_sample;
        let cwnd_sample;
        {
            flowwnd_sample = *flowwnd.lock().unwrap();
        }
        {
            cwnd_sample = *cwnd.lock().unwrap();
        }
        if total_bytes - last_ackd_bytes >= min(flowwnd_sample, cwnd_sample.0) as u64 {
            let mut illegal_ack = false;

            {
                let (lock, cvar) = &*last_ackd_ids;
                let mut ids = lock.lock().unwrap();
                loop {
                    if ids[0] > last_ackd_pckt_id {
                        //new ACK received
                        //spool forward bytes received
                        for i in (last_ackd_pckt_id + 1)..(ids[0] + 1) {
                            last_ackd_bytes += ringbuf_szs[(i as usize) % ringbuf_size] as u64;
                        }
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
                        total_bytes = last_ackd_bytes;
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

            //wait unboundedly long for fist frame
            let frame = if !peeked_frame.is_empty() {
                peeked_frame.pop().unwrap()
            } else {
                match mux_rx.next().await {
                    None => return Ok(()),
                    Some(f) => f,
                }
            };

            loop {
                //TODO: how long to wait for more frames?
                //wait a short time for further frames
                let frame = match timeout(Duration::from_millis(1), mux_rx.next()).await {
                    Ok(fo) => match fo {
                        None => {
                            return Ok(());
                        }
                        Some(f) => f,
                    },
                    Err(_) => {
                        //send packet if no next frame arrives in time
                        break;
                    }
                };

                //check if max size surpassed -> save overhanging frame and break
                if size + frame.size() > max_packet_size {
                    peeked_frame.push(frame);
                    break;
                }

                size += packet.size(); //TODO how to measure actual size?
                packet.add_frame(frame);
            }

            //insert packet size to packet size ring buffer
            ringbuf_szs[((packet_id + 1) as usize) % ringbuf_size] = packet.size() as u32;

            //insert packet to ring buffer
            ringbuf_pkts[((packet_id + 1) as usize) % ringbuf_size] = packet.clone();
            //TODO: delete packets out of window to save memory
        } else {
            //resend from ring buffer
            packet = ringbuf_pkts[((tx_packet_id + 1) as usize) % ringbuf_size].clone();
        }

        total_bytes += packet.size() as u64;

        //send packet trough sink
        sink.send(packet).await.expect("could not send packet");

        //if rewinding, increment only tx_packet_id
        if packet_id == tx_packet_id {
            packet_id += 1;
        }
        tx_packet_id += 1;
    }
}
