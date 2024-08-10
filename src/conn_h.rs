use crate::wire::{AckFrame, AnswerFrame, AnswerHeader, ErrorFrame, ErrorHeader, Frame, Frames};
use anyhow::{anyhow, Result};
use bytes::{Bytes, BytesMut};
use futures::{Sink, SinkExt, Stream, StreamExt};
use std::fmt::Debug;
use std::io::{BufReader, BufWriter, Read, Write};
use std::{fs, str};
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
fn make_error(stream_id: u16, frame_id: u32, msg: String) -> Frame {
    let bytes_msg = Bytes::from(msg);
    let payload_len_be_bytes: &[u8] = &(bytes_msg.len() as u16).to_be_bytes();
    let mut payload_m = BytesMut::from(payload_len_be_bytes);
    payload_m.extend(bytes_msg);
    let payload: Bytes = payload_m.into();
    ErrorFrame {
        header: &ErrorHeader {
            typ: 5,
            stream_id,
            frame_id: frame_id + 1,
            command_frame_id: frame_id,
        },
        payload: &payload,
    }
    .into()
}

#[allow(dead_code)]
pub async fn stream_handler<S: Sink<Frame> + Unpin>(
    mut stream: impl Stream<Item = Frames<'_>> + Unpin,
    mut sink: S,
) -> anyhow::Result<()>
where
    <S as futures::Sink<Frame>>::Error: Debug,
{
    match stream.next().await {
        None => Ok(()),
        Some(frame) => {
            match frame {
                Frames::Read(cmd) => {
                    sink.send(make_error(
                        cmd.header.stream_id,
                        cmd.header.frame_id,
                        "Not implemented".into(),
                    ))
                    .await
                    .expect("stream_handler: could not send response");
                    Ok(())
                }

                Frames::Write(cmd) => {
                    //parse path
                    let path: String = match str::from_utf8(cmd.payload) {
                        Ok(s) => s.into(),
                        Err(_) => {
                            sink.send(make_error(
                                cmd.header.stream_id,
                                cmd.header.frame_id,
                                "Invalid Payload".into(),
                            ))
                            .await
                            .expect("stream_handler: could not send response");
                            return Ok(());
                        }
                    };

                    //create / open file
                    let file: File = match OpenOptions::new()
                        .write(true)
                        .create(true)
                        .truncate(false)
                        .open(path.clone())
                    {
                        Ok(f) => f,
                        Err(e) => {
                            sink.send(make_error(
                                cmd.header.stream_id,
                                cmd.header.frame_id,
                                e.to_string(),
                            ))
                            .await
                            .expect("stream_handler: could not send response");
                            return Ok(());
                        }
                    };

                    //check if file size matches write offset
                    let metadata = fs::metadata(path.clone()).expect("Could not get file metadata");
                    if metadata.len() != cmd.header.offset() {
                        sink.send(make_error(
                            cmd.header.stream_id,
                            cmd.header.frame_id,
                            "Write offset does not match file size".into(),
                        ))
                        .await
                        .expect("stream_handler: could not send response");
                        return Ok(());
                    }

                    //send ACK for command
                    sink.send(AckFrame {
                        typ: 0,
                        stream_id: cmd.header.stream_id,
                        frame_id: cmd.header.frame_id,
                    }.into()).await.expect("stream_handler: could not send response".into());

                    //receive Data frames and write to file; stop if transmission complete
                    let mut writer = BufWriter::new(file);
                    let mut last_offset = cmd.header.offset();
                    let mut last_frame_id = cmd.header.frame_id;
                    let mut cum_ack_ctr = 0;
                    loop {
                        let next_frame = match timeout(Duration::from_secs(5), stream.next()).await
                        {
                            Ok(f) => f,
                            Err(_) => {
                                //timeout: sed error frame, exit
                                sink.send(make_error(
                                    cmd.header.stream_id,
                                    last_frame_id,
                                    "Timeout".into(),
                                ))
                                .await
                                .expect("stream_handler: could not send response");
                                return Ok(());
                            }
                        };

                        if let Some(Frames::Data(f)) = next_frame {
                            //empty data frame marks end of transmission
                            if f.header.length() == 0 {
                                sink.send(
                                    AckFrame {
                                        typ: 0,
                                        stream_id: cmd.header.stream_id,
                                        frame_id: f.header.frame_id,
                                    }
                                    .into(),
                                )
                                .await
                                .expect("stream_handler: could not send ACK");
                                break;
                            }

                            //check if offset matches
                            if last_offset != f.header.offset() {
                                //mismatch -> send double ACK, discard packet
                                sink.send(
                                    AckFrame {
                                        typ: 0,
                                        stream_id: cmd.header.stream_id,
                                        frame_id: last_frame_id,
                                    }
                                    .into(),
                                )
                                .await
                                .expect("stream_handler: could not send ACK");

                                cum_ack_ctr = 0;
                                continue;
                            }

                            //write data from frame to file
                            writer
                                .write_all(f.payload)
                                .expect("Could not write to BufWriter");

                            //update last received frame id and offset
                            last_frame_id = f.header.frame_id;
                            last_offset += f.header.length();

                            //send ACK
                            cum_ack_ctr += 1;
                            if cum_ack_ctr >= 2 {
                                //TODO: how to determine cumulative ACK interval?   (caution: dependency in tests)
                                /*
                                //flush, so that the send ACK actually reports successful write
                                writer.flush().expect("Could not flush to file");
                                 */

                                sink.send(
                                    AckFrame {
                                        typ: 0,
                                        stream_id: cmd.header.stream_id,
                                        frame_id: f.header.frame_id,
                                    }
                                    .into(),
                                )
                                .await
                                .expect("stream_handler: could not send ACK");

                                cum_ack_ctr = 0;
                            }
                        } else {
                            //illegal frame or channel closed: abort transmission and leave file so client can continue later
                            sink.send(make_error(
                                cmd.header.stream_id,
                                0,
                                "Illegal Frame Received".into(),
                            ))
                            .await
                            .expect("stream_handler: could not send response");
                            return Ok(());
                        }
                    }
                    Ok(())
                }

                Frames::Checksum(cmd) => {
                    match str::from_utf8(cmd.payload) {
                        Ok(p) => match File::open(p) {
                            Ok(f) => {
                                let reader = BufReader::new(f);
                                let digest = sha256_digest(reader)?;
                                sink.send(
                                    AnswerFrame {
                                        header: &AnswerHeader {
                                            typ: 4,
                                            stream_id: cmd.header.stream_id,
                                            frame_id: cmd.header.frame_id + 1,
                                            command_frame_id: cmd.header.frame_id,
                                        },
                                        payload: &Bytes::copy_from_slice(digest.as_ref()),
                                    }
                                    .into(),
                                )
                                .await
                                .expect("stream_handler: could not send response");
                            }
                            Err(e) => {
                                sink.send(make_error(
                                    cmd.header.stream_id,
                                    cmd.header.frame_id,
                                    e.to_string(),
                                ))
                                .await
                                .expect("stream_handler: could not send response");
                                return Ok(());
                            }
                        },
                        Err(_) => {
                            sink.send(make_error(
                                cmd.header.stream_id,
                                cmd.header.frame_id,
                                "Invalid Payload".into(),
                            ))
                            .await
                            .expect("stream_handler: could not send response");
                            return Ok(());
                        }
                    }

                    Ok(())
                }

                Frames::Stat(cmd) => {
                    sink.send(make_error(
                        cmd.header.stream_id,
                        cmd.header.frame_id,
                        "Not implemented".into(),
                    ))
                    .await
                    .expect("stream_handler: could not send response");
                    Ok(())
                }

                Frames::List(cmd) => {
                    sink.send(make_error(
                        cmd.header.stream_id,
                        cmd.header.frame_id,
                        "Not implemented".into(),
                    ))
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
    use crate::wire::{ChecksumFrame, ChecksumHeader, DataFrame, DataHeader, WriteFrame, WriteHeader};
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
            offset: [0,0,0,0,0,0],
            length: lfd1_b6,
        };

        let data2_hdr: DataHeader = DataHeader {
            typ: 6,
            stream_id: req_hd.stream_id,
            frame_id: 3,
            offset: [0,0,0,0,0,128],
            length: lfd2_b6,
        };

        let data3_hdr: DataHeader = DataHeader {
            typ: 6,
            stream_id: req_hd.stream_id,
            frame_id: 4,
            offset: [0,0,0,0,1,78],
            length: [0,0,0,0,0,0],
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
                payload: &dp1_bytes
            })).await.unwrap();

            itx.send(Frames::Data(DataFrame {
                header: &data2_hdr,
                payload: &dp2_bytes
            })).await.unwrap();

            //send EOF frame
            itx.send(Frames::Data(DataFrame {
                header: &data3_hdr,
                payload: &dp3_bytes
            })).await.unwrap();

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
}
