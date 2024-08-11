use crate::wire::{
    AckFrame, AnswerFrame, DataFrame, ErrorFrame, Frame,
};
use anyhow::{anyhow, Result};
use bytes::Bytes;
use futures::{Sink, SinkExt, Stream, StreamExt};
use std::cmp::Ordering;
use std::fmt::Debug;
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::fs;
use tokio::time::timeout;

use ring::digest;
use ring::digest::{Digest, SHA256};
use std::fs::{File, OpenOptions};
use std::time::Duration;

//from rust cookbook
#[allow(dead_code)]
fn sha256_digest<R: Read>(mut reader: R) -> Result<Digest> {
    let mut context = digest::Context::new(&SHA256);
    let mut buffer = [0; 1024];

    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        context.update(&buffer[..count]);
    }

    Ok(context.finish())
}

#[allow(dead_code)]
pub async fn stream_handler<S: Sink<Frame> + Unpin>(
    mut stream: impl Stream<Item = Frame> + Unpin,
    mut sink: S,
) -> anyhow::Result<()>
where
    <S as futures::Sink<Frame>>::Error: Debug,
{
    match stream.next().await {
        None => Ok(()),
        Some(frame) => {
            match frame {
                Frame::Read(cmd) => {
                    //for now: just jam frames into pipe until pipe is full or cum. ACK limit reached
                    //TODO: how and where to handle flow control?

                    //last sent frame number (increment before use)
                    let mut frame_number: u32 = 0;

                    //parse path
                    let path: String = match cmd.path().to_str() {
                        Some(s) => s.into(),
                        None => {
                            frame_number += 1;
                            sink.send(ErrorFrame::new(
                                cmd.stream_id(),
                                cmd.frame_id(),
                                frame_number,
                                "Invalid Payload".into(),
                            ).into())
                            .await
                            .expect("stream_handler: could not send response");
                            return Ok(());
                        }
                    };

                    //open file
                    let file: File = match OpenOptions::new().read(true).open(path.clone()) {
                        Ok(f) => f,
                        Err(e) => {
                            frame_number += 1;
                            sink.send(ErrorFrame::new(
                                cmd.stream_id(),
                                cmd.frame_id(),
                                frame_number,
                                e.to_string().as_str(),
                            ).into())
                            .await
                            .expect("stream_handler: could not send response");
                            return Ok(());
                        }
                    };

                    //get file size
                    let metadata = fs::metadata(path.clone()).expect("Could not get file metadata");
                    let file_size = metadata.len();

                    //TODO: actually stop reading at cmd.header.length()

                    //move cursor to offset
                    //TODO: may have to check manually if offset is past file size ??
                    let mut reader = BufReader::new(file);
                    match reader.seek(SeekFrom::Start(cmd.offset())) {
                        Ok(_) => {}
                        Err(e) => {
                            frame_number += 1;
                            sink.send(ErrorFrame::new(
                                cmd.stream_id(),
                                cmd.frame_id(),
                                frame_number,
                                e.to_string().as_str(),
                            ).into())
                            .await
                            .expect("stream_handler: could not send response");
                            return Ok(());
                        }
                    }

                    //send ACK for command
                    sink.send(
                        AckFrame::new(
                            cmd.stream_id(),
                            cmd.frame_id()
                        )
                        .into(),
                    )
                    .await
                    .expect("stream_handler: could not send response");

                    //read data from file and generate data frames
                    let cum_ack_interval: u32 = 2; //TODO: how to determine cum. ACK interval?
                    let mut last_offset = cmd.offset(); //the first byte not yet sent
                    let mut last_ackd_frame = frame_number;
                    let mut last_ackd_offset = cmd.offset(); //the first not yet ACK'd byte

                    //this buffer includes, for the last x frames sent, up to (exclusive) which offset the frame contained data
                    let mut cum_ack_offset_ringbuf = vec![0u64; cum_ack_interval as usize];
                    let mut ringbuf_head: u32 = 0; //head: last written element (increment before use)

                    //TODO: which buf size to use? 128 for tests.
                    let mut read_buf = [0u8; 128];
                    loop {
                        //check if we need to wait for ack
                        if (frame_number - last_ackd_frame >= cum_ack_interval)
                            || last_offset > file_size
                        {
                            //receive or wait for ACK frame
                            match timeout(Duration::from_secs(5), stream.next()).await {
                                Ok(Some(Frame::Ack(af))) => {
                                    //check if new ack, double ack, or illegal
                                    let af_frame_id = af.frame_id();
                                    match af_frame_id.cmp(&last_ackd_frame) {
                                        Ordering::Greater => {
                                            //new ACK: advance last ack'd frame id and offset
                                            last_ackd_offset = cum_ack_offset_ringbuf
                                                [(((ringbuf_head + cum_ack_interval)
                                                    - (af.frame_id() - last_ackd_frame))
                                                    % cum_ack_interval)
                                                    as usize];
                                            last_ackd_frame = af.frame_id();
                                        }
                                        Ordering::Equal => {
                                            //double ACK: rewind reader to last ACK'd offset
                                            reader
                                                .seek(SeekFrom::Start(last_ackd_offset))
                                                .expect("file read error");
                                            //rewind frame number
                                            frame_number = last_ackd_frame;
                                        }
                                        Ordering::Less => {
                                            //"rewind ACK" ???????
                                            frame_number += 1;
                                            sink.send(ErrorFrame::new(
                                                cmd.stream_id(),
                                                af_frame_id,
                                                frame_number,
                                                "ACK'd frame number inconsistency".into(),
                                            ).into())
                                            .await
                                            .expect("stream_handler: could not send response");
                                            return Ok(());
                                        }
                                    }
                                }
                                Err(_) => {
                                    //timeout: send error frame, exit
                                    //TODO: retry x times
                                    frame_number += 1;
                                    sink.send(ErrorFrame::new(
                                        cmd.stream_id(),
                                        cmd.frame_id(),
                                        frame_number,
                                        "Timeout".into(),
                                    ).into())
                                    .await
                                    .expect("stream_handler: could not send response");
                                    return Ok(());
                                }
                                _ => {
                                    //other frame received: send error frame, exit
                                    frame_number += 1;
                                    sink.send(ErrorFrame::new(
                                        cmd.stream_id(),
                                        cmd.frame_id(),
                                        frame_number,
                                        "Illegal Frame Received".into(),
                                    ).into())
                                    .await
                                    .expect("stream_handler: could not send response");
                                    return Ok(());
                                }
                            };
                        }

                        //check if we are finished
                        //TODO: is this actually guaranteed to work??
                        if last_ackd_offset >= file_size {
                            break;
                        };

                        //read bytes from file into buf
                        let data_size = reader.read(&mut read_buf).expect("file read error");
                        let data_bytes = Bytes::copy_from_slice(&read_buf[..data_size]);

                        //assemble and dispatch data frame
                        {
                            sink.send(
                                DataFrame::new(cmd.stream_id(), frame_number + 1, last_offset, data_bytes)
                                .into()
                            )
                            .await
                            .expect("stream_handler: could not send response");
                        }

                        //update counters
                        frame_number += 1;
                        last_offset += data_size as u64;

                        //advance ring buffer
                        ringbuf_head = (ringbuf_head + 1) % cum_ack_interval;
                        cum_ack_offset_ringbuf[ringbuf_head as usize] = last_offset;
                    }

                    Ok(())
                }

                Frame::Write(cmd) => {
                    let mut frame_number = 0; //last used tx frame number

                    //parse path
                    let path: String = match cmd.path().to_str() {
                        Some(s) => s.into(),
                        None => {
                            frame_number += 1;
                            sink.send(ErrorFrame::new(
                                cmd.stream_id(),
                                cmd.frame_id(),
                                frame_number,
                                "Invalid Payload".into(),
                            ).into())
                            .await
                            .expect("stream_handler: could not send response");
                            return Ok(());
                        }
                    };

                    //create / open file
                    //TODO: use cmd-header.length() to check if enough disk space available
                    let file: File = match OpenOptions::new()
                        .write(true)
                        .create(true)
                        .truncate(false)
                        .open(path.clone())
                    {
                        Ok(f) => f,
                        Err(e) => {
                            frame_number += 1;
                            sink.send(ErrorFrame::new(
                                cmd.stream_id(),
                                cmd.frame_id(),
                                frame_number,
                                e.to_string().as_str(),
                            ).into())
                            .await
                            .expect("stream_handler: could not send response");
                            return Ok(());
                        }
                    };

                    //check if file size matches write offset
                    let metadata = fs::metadata(path.clone()).expect("Could not get file metadata");
                    if metadata.len() != cmd.offset() {
                        frame_number += 1;
                        sink.send(ErrorFrame::new(
                            cmd.stream_id(),
                            cmd.frame_id(),
                            frame_number,
                            "Write offset does not match file size".into(),
                        ).into())
                        .await
                        .expect("stream_handler: could not send response");
                        return Ok(());
                    }

                    //send ACK for command
                    sink.send(
                        AckFrame::new(
                            cmd.stream_id(),
                            cmd.frame_id(),
                        )
                        .into(),
                    )
                    .await
                    .expect("stream_handler: could not send response");

                    //receive Data frames and write to file; stop if transmission complete
                    let mut writer = BufWriter::new(file);
                    let mut last_offset = cmd.offset();
                    let mut last_frame_id = cmd.frame_id();
                    let mut cum_ack_ctr = 0;
                    loop {
                        let next_frame = match timeout(Duration::from_secs(5), stream.next()).await
                        {
                            Ok(f) => f,
                            Err(_) => {
                                //timeout: sed error frame, exit
                                frame_number += 1;
                                sink.send(ErrorFrame::new(
                                    cmd.stream_id(),
                                    last_frame_id,
                                    frame_number,
                                    "Timeout".into(),
                                ).into())
                                .await
                                .expect("stream_handler: could not send response");
                                return Ok(());
                            }
                        };

                        if let Some(Frame::Data(f)) = next_frame {
                            //empty data frame marks end of transmission
                            if f.length() == 0 {
                                sink.send(
                                    AckFrame::new(
                                        cmd.stream_id(),
                                        f.frame_id()
                                    )
                                    .into(),
                                )
                                .await
                                .expect("stream_handler: could not send ACK");
                                break;
                            }

                            //check if offset matches
                            if last_offset != f.offset() {
                                //mismatch -> send double ACK, discard packet
                                sink.send(
                                    AckFrame::new(
                                        cmd.stream_id(),
                                        last_frame_id,
                                    )
                                    .into(),
                                )
                                .await
                                .expect("stream_handler: could not send ACK");

                                cum_ack_ctr = 0;
                                continue;
                            }

                            //write data from frame to file
                            writer
                                .write_all(f.payload())
                                .expect("Could not write to BufWriter");

                            //update last received frame id and offset
                            last_frame_id = f.frame_id();
                            last_offset += f.length();

                            //send ACK
                            cum_ack_ctr += 1;
                            if cum_ack_ctr >= 2 {
                                //TODO: how to determine cumulative ACK interval?   (caution: dependency in tests)
                                /*
                                //flush, so that the send ACK actually reports successful write
                                writer.flush().expect("Could not flush to file");
                                 */

                                sink.send(
                                    AckFrame::new(
                                        cmd.stream_id(),
                                        f.frame_id(),
                                    )
                                    .into(),
                                )
                                .await
                                .expect("stream_handler: could not send ACK");

                                cum_ack_ctr = 0;
                            }
                        } else {
                            //illegal frame or channel closed: abort transmission and leave file so client can continue later
                            frame_number += 1;
                            sink.send(ErrorFrame::new(
                                cmd.stream_id(),
                                cmd.frame_id(),
                                frame_number,
                                "Illegal Frame Received".into(),
                            ).into())
                            .await
                            .expect("stream_handler: could not send response");
                            return Ok(());
                        }
                    }
                    Ok(())
                }

                Frame::Checksum(cmd) => {
                    match cmd.path().to_str() {
                        Some(p) => match File::open(p) {
                            Ok(f) => {
                                let reader = BufReader::new(f);
                                let digest = sha256_digest(reader)?;
                                sink.send(
                                    AnswerFrame::new(
                                            cmd.stream_id(),
                                            cmd.frame_id() + 1,
                                            cmd.frame_id(),
                                            Bytes::copy_from_slice(digest.as_ref()),
                                    )
                                    .into(),
                                )
                                .await
                                .expect("stream_handler: could not send response");
                            }
                            Err(e) => {
                                sink.send(ErrorFrame::new(
                                    cmd.stream_id(),
                                    cmd.frame_id(),
                                    1,
                                    e.to_string().as_str(),
                                ).into())
                                .await
                                .expect("stream_handler: could not send response");
                                return Ok(());
                            }
                        },
                        None => {
                            sink.send(ErrorFrame::new(
                                cmd.stream_id(),
                                cmd.frame_id(),
                                1,
                                "Invalid Payload".into(),
                            ).into())
                            .await
                            .expect("stream_handler: could not send response");
                            return Ok(());
                        }
                    }

                    Ok(())
                }

                Frame::Stat(cmd) => {
                    sink.send(ErrorFrame::new(
                        cmd.stream_id(),
                        cmd.frame_id(),
                        1,
                        "Not implemented".into(),
                    ).into())
                    .await
                    .expect("stream_handler: could not send response");
                    Ok(())
                }

                Frame::List(cmd) => {
                    sink.send(ErrorFrame::new(
                        cmd.stream_id(),
                        cmd.frame_id(),
                        1,
                        "Not implemented".into(),
                    ).into())
                    .await
                    .expect("stream_handler: could not send response");
                    Ok(())
                }

                _ => Err(anyhow!("Illegal initial frame reached stream_handler")),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wire::Frames::{Checksum, Error};
    use crate::wire::{
        ChecksumFrame, ChecksumHeader, DataFrame, DataHeader, ReadFrame, ReadHeader, WriteFrame,
        WriteHeader,
    };
    use crate::wire::{Frames, Frames::Answer};
    use data_encoding::HEXLOWER;
    use futures::channel::mpsc::{channel, Receiver, Sender};

    #[tokio::test]
    async fn test_checksum() {
        let path = "testfile.txt";
        let mut out = File::create(path).unwrap();
        write!(out, "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.").unwrap();
        let payload = Bytes::copy_from_slice(path.as_bytes());

        {
            let (mut itx, irx): (Sender<Frames>, Receiver<Frames>) = channel(1);
            let (otx, mut orx): (Sender<Frame>, Receiver<Frame>) = channel(1);
            itx.send(Checksum(ChecksumFrame {
                header: &ChecksumHeader {
                    typ: 9,
                    stream_id: 420,
                    frame_id: 1,
                },
                payload: &payload,
            }))
            .await
            .unwrap();

            match stream_handler(irx, otx).await {
                Ok(()) => {
                    let f = orx.next().await.unwrap();
                    let af = f.header();

                    match af {
                        Answer(a) => {
                            assert_eq!(a.header.typ, 4);
                            let sid = a.header.stream_id;
                            assert_eq!(sid, 420);
                            let fid = a.header.frame_id;
                            assert_eq!(fid, 2);
                            let cid = a.header.command_frame_id;
                            assert_eq!(cid, 1);

                            let s = HEXLOWER.encode(a.payload);

                            //reference hash computed with 7zip
                            assert_eq!(
                                s,
                                "973153f86ec2da1748e63f0cf85b89835b42f8ee8018c549868a1308a19f6ca3"
                            );
                        }
                        _ => {
                            assert!(false)
                        }
                    }
                }
                Err(_) => {
                    assert!(false);
                }
            }

            fs::remove_file(path).unwrap();
        }
    }

    #[tokio::test]
    async fn test_error() {
        let path = "err_testfile.txt";
        //let mut out = File::create(path).unwrap();
        //write!(out, "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.").unwrap();
        let payload = Bytes::copy_from_slice(path.as_bytes());

        {
            let (mut itx, irx): (Sender<Frames>, Receiver<Frames>) = channel(1);
            let (otx, mut orx): (Sender<Frame>, Receiver<Frame>) = channel(1);
            itx.send(Checksum(ChecksumFrame {
                header: &ChecksumHeader {
                    typ: 9,
                    stream_id: 420,
                    frame_id: 1,
                },
                payload: &payload,
            }))
            .await
            .unwrap();

            match stream_handler(irx, otx).await {
                Ok(()) => {
                    let f = orx.next().await.unwrap();
                    let af = f.header();

                    match af {
                        Error(e) => {
                            let msg_hex = HEXLOWER.encode(e.payload);
                            //let msg = str::from_utf8(e.payload).unwrap();
                            //error message: "No such file or directory (os error 2)", preceded by "0026" for length 38 (hex 0x26) in big endian
                            assert_eq!(msg_hex, "00264e6f20737563682066696c65206f72206469726563746f727920286f73206572726f72203229");
                        }
                        _ => {
                            assert!(false)
                        }
                    }
                }
                Err(_) => {
                    assert!(false);
                }
            }
        }
    }

    #[tokio::test]
    async fn test_write_new_file() {
        //name and contents of file to write
        let path = "twnf_testfile.txt";
        let payload = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur.";

        //prepare headers and data for frames to be sent
        let path_bytes = Bytes::copy_from_slice(path.as_bytes());
        let lfh_b8: [u8; 8] = u64::to_be_bytes(path_bytes.len() as u64);
        let lfh_b6: [u8; 6] = lfh_b8[2..].try_into().unwrap();

        let dp1_bytes = Bytes::copy_from_slice(&payload.as_bytes()[..128]);
        let lfd1_b8: [u8; 8] = u64::to_be_bytes(dp1_bytes.len() as u64);
        let lfd1_b6: [u8; 6] = lfd1_b8[2..].try_into().unwrap();

        let dp2_bytes = Bytes::copy_from_slice(&payload.as_bytes()[128..]);
        let lfd2_b8: [u8; 8] = u64::to_be_bytes(dp2_bytes.len() as u64);
        let lfd2_b6: [u8; 6] = lfd2_b8[2..].try_into().unwrap();

        let dp3_bytes = Bytes::default();

        let req_hd: WriteHeader = WriteHeader {
            typ: 8,
            stream_id: 420,
            frame_id: 1,
            offset: [0, 0, 0, 0, 0, 0],
            length: lfh_b6,
        };

        let data1_hdr: DataHeader = DataHeader {
            typ: 6,
            stream_id: req_hd.stream_id,
            frame_id: 2,
            offset: [0, 0, 0, 0, 0, 0],
            length: lfd1_b6,
        };

        let data2_hdr: DataHeader = DataHeader {
            typ: 6,
            stream_id: req_hd.stream_id,
            frame_id: 3,
            offset: [0, 0, 0, 0, 0, 128],
            length: lfd2_b6,
        };

        let data3_hdr: DataHeader = DataHeader {
            typ: 6,
            stream_id: req_hd.stream_id,
            frame_id: 4,
            offset: [0, 0, 0, 0, 1, 78],
            length: [0, 0, 0, 0, 0, 0],
        };

        {
            let (mut itx, irx): (Sender<Frames>, Receiver<Frames>) = channel(5);
            let (otx, mut orx): (Sender<Frame>, Receiver<Frame>) = channel(5);

            //send command frame
            itx.send(Frames::Write(WriteFrame {
                header: &req_hd,
                payload: &path_bytes,
            }))
            .await
            .unwrap();

            //send data frames
            itx.send(Frames::Data(DataFrame {
                header: &data1_hdr,
                payload: &dp1_bytes,
            }))
            .await
            .unwrap();

            itx.send(Frames::Data(DataFrame {
                header: &data2_hdr,
                payload: &dp2_bytes,
            }))
            .await
            .unwrap();

            //send EOF frame
            itx.send(Frames::Data(DataFrame {
                header: &data3_hdr,
                payload: &dp3_bytes,
            }))
            .await
            .unwrap();

            //run handler and test whether 3x ACK received and file written
            match stream_handler(irx, otx).await {
                Ok(()) => {
                    let f1 = orx.next().await.unwrap();
                    let fh1 = f1.header();

                    match fh1 {
                        Frames::Ack(a) => {
                            let afid = a.frame_id;
                            let reqfid = req_hd.frame_id;
                            assert_eq!(afid, reqfid);

                            let asid = a.stream_id;
                            let reqsid = req_hd.stream_id;
                            assert_eq!(asid, reqsid);
                        }
                        _ => {
                            assert!(false)
                        }
                    }

                    let f2 = orx.next().await.unwrap();
                    let fh2 = f2.header();

                    match fh2 {
                        Frames::Ack(a) => {
                            let afid = a.frame_id;
                            let dt2fid = data2_hdr.frame_id;
                            assert_eq!(afid, dt2fid);

                            let asid = a.stream_id;
                            let reqsid = req_hd.stream_id;
                            assert_eq!(asid, reqsid);
                        }
                        _ => {
                            assert!(false)
                        }
                    }

                    let f3 = orx.next().await.unwrap();
                    let fh3 = f3.header();

                    match fh3 {
                        Frames::Ack(a) => {
                            let afid = a.frame_id;
                            let dt3fid = data3_hdr.frame_id;
                            assert_eq!(afid, dt3fid);

                            let asid = a.stream_id;
                            let reqsid = req_hd.stream_id;
                            assert_eq!(asid, reqsid);
                        }
                        _ => {
                            assert!(false)
                        }
                    }

                    //check file
                    let file_str = fs::read_to_string(path).unwrap();
                    assert_eq!(file_str, payload);
                }
                Err(_) => {
                    assert!(false);
                }
            }

            fs::remove_file(path).unwrap();
        }
    }

    #[tokio::test]
    async fn test_read_off0_with_write() {
        //this is a simple test testing reading a whole file with no complications

        //create file
        let path = "testfile_r.txt";
        let mut out = File::create(path).unwrap();
        let file_text = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur.";
        write!(out, "{}", file_text).unwrap();

        //create read and write commands
        let payload_r = Bytes::copy_from_slice(path.as_bytes());

        {
            let (mut itx, irx): (Sender<Frames>, Receiver<Frames>) = channel(4);
            let (otx, mut orx): (Sender<Frame>, Receiver<Frame>) = channel(5);

            //send read command
            itx.send(Frames::Read(ReadFrame {
                header: &ReadHeader {
                    typ: 7,
                    stream_id: 69,
                    frame_id: 1,
                    flags: 0,
                    offset: [0, 0, 0, 0, 0, 0],
                    length: [0, 0, 0, 0, 0, 0],
                    checksum: 0,
                },
                payload: &payload_r,
            }))
            .await
            .unwrap();

            //send one ACK for packet 2
            itx.send(Frames::Ack(&AckFrame {
                typ: 0,
                stream_id: 69,
                frame_id: 2,
            }))
            .await
            .unwrap();

            //send one ACK for packet 4
            itx.send(Frames::Ack(&AckFrame {
                typ: 0,
                stream_id: 69,
                frame_id: 4,
            }))
            .await
            .unwrap();

            //send one ACK for last packet (5)
            itx.send(Frames::Ack(&AckFrame {
                typ: 0,
                stream_id: 69,
                frame_id: 5,
            }))
            .await
            .unwrap();

            let mut rec = String::new();

            //start handler
            match stream_handler(irx, otx).await {
                Ok(_) => {
                    //receive one ACK and three data frames + EOF, check whether contents are correct
                    let f1 = orx.next().await.unwrap();
                    let fh1 = f1.header();

                    match fh1 {
                        Frames::Ack(a) => {
                            let afid = a.frame_id;
                            let reqfid = 1;
                            assert_eq!(afid, reqfid);

                            let asid = a.stream_id;
                            let reqsid = 69;
                            assert_eq!(asid, reqsid);
                        }
                        _ => {
                            assert!(false)
                        }
                    }

                    let f2 = orx.next().await.unwrap();
                    let fh2 = f2.header();

                    match fh2 {
                        Frames::Data(d) => rec.push_str(str::from_utf8(d.payload).unwrap()),
                        _ => {
                            assert!(false)
                        }
                    }

                    let f3 = orx.next().await.unwrap();
                    let fh3 = f3.header();

                    match fh3 {
                        Frames::Data(d) => rec.push_str(str::from_utf8(d.payload).unwrap()),
                        _ => {
                            assert!(false)
                        }
                    }

                    let f4 = orx.next().await.unwrap();
                    let fh4 = f4.header();

                    match fh4 {
                        Frames::Data(d) => rec.push_str(str::from_utf8(d.payload).unwrap()),
                        _ => {
                            assert!(false)
                        }
                    }

                    //EOF
                    let f5 = orx.next().await.unwrap();
                    let fh5 = f5.header();

                    match fh5 {
                        Frames::Data(d) => {
                            assert_eq!(d.header.length(), 0);
                        }
                        _ => {
                            assert!(false)
                        }
                    }

                    match orx.next().await {
                        None => {}
                        Some(_) => {
                            assert!(false);
                        }
                    }

                    //check contents
                    assert_eq!(rec.as_str(), file_text);
                }
                Err(_) => {
                    assert!(false);
                }
            }

            fs::remove_file(path).unwrap();
        }
    }
}
