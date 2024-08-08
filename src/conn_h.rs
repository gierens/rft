use futures::{Sink, SinkExt, Stream, StreamExt};
use std::{fs, str};
use std::fmt::Debug;
use std::io::{Read, Write, BufReader};
use anyhow::{Result, anyhow};
use bytes::{Bytes};
use crate::wire::{AnswerFrame, AnswerHeader, ErrorFrame, ErrorHeader, Frame, Frames};
use crate::wire::Frames::{Answer};

use ring::digest;
use ring::digest::{Digest, SHA256};
use std::fs::File;

//from rust cookbook
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

fn make_error (stream_id: u16, frame_id: u32, msg: String) -> Frame {
    ErrorFrame { header: &ErrorHeader {
        typ: 5,
        stream_id,
        frame_id: frame_id + 1,
        command_frame_id: frame_id,
    }, payload: &Bytes::from(msg)
    }.into()
}

pub async fn stream_handler<S: Sink<Frame> + Unpin>(mut stream: impl Stream<Item = Frames<'_>> + Unpin, mut sink: S) -> anyhow::Result<()> where <S as futures::Sink<Frame>>::Error: Debug{
    match stream.next().await {
        None => {Ok(())}
        Some(frame) => {
            match frame {
                Frames::Read(cmd) => {
                    sink.send(make_error(cmd.header.stream_id, cmd.header.frame_id, "Not implemented".into())).await.expect("stream_handler: could not send response");
                    Ok(())
                }
                Frames::Write(cmd) => {
                    sink.send(make_error(cmd.header.stream_id, cmd.header.frame_id, "Not Implemented".into())).await.expect("stream_handler: could not send response");
                    Ok(())
                }
                Frames::Checksum(cmd) => {
                    match str::from_utf8(cmd.payload) {
                        Ok(p) => {
                            match File::open(p) {
                                Ok(f) => {
                                    let reader = BufReader::new(f);
                                    let digest = sha256_digest(reader)?;
                                    sink.send(AnswerFrame {
                                        header: &AnswerHeader {
                                            typ: 4,
                                            stream_id: cmd.header.stream_id,
                                            frame_id: cmd.header.frame_id + 1,
                                            command_frame_id: cmd.header.frame_id,
                                        },
                                        payload: &Bytes::copy_from_slice(digest.as_ref())
                                    }.into()).await.expect("stream_handler: could not send response");
                                }
                                Err(e) => {
                                    sink.send(make_error(cmd.header.stream_id, cmd.header.frame_id, e.to_string())).await.expect("stream_handler: could not send response");
                                    return Ok(())
                                }
                            }
                        }
                        Err(_) => {
                            sink.send(make_error(cmd.header.stream_id, cmd.header.frame_id, "Invalid Payload".into())).await.expect("stream_handler: could not send response");
                            return Ok(())
                        }
                    }

                    Ok(())
                }
                Frames::Stat(cmd) => {
                    sink.send(make_error(cmd.header.stream_id, cmd.header.frame_id, "Not implemented".into())).await.expect("stream_handler: could not send response");
                    Ok(())
                }
                Frames::List(cmd) => {
                    sink.send(make_error(cmd.header.stream_id, cmd.header.frame_id, "Not implemented".into())).await.expect("stream_handler: could not send response");
                    Ok(())
                }
                _ => {Err(anyhow!("Illegal initial frame reached stream_handler"))}
            }
        }
    }
}

mod tests {
    use futures::channel::mpsc::{channel, Receiver, Sender};
    use crate::wire::{ChecksumFrame, ChecksumHeader};
    use crate::wire::Frames::Checksum;
    use data_encoding::HEXLOWER;
    #[allow(unused_imports)]
    use super::*;

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
                    typ: 69,
                    stream_id: 420,
                    frame_id: 1,
                },
                payload: &payload
            })).await.unwrap();

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
                            assert_eq!(s, "973153f86ec2da1748e63f0cf85b89835b42f8ee8018c549868a1308a19f6ca3");
                        }
                        _ => { assert!(false) }
                    }
                }
                Err(_) => {
                    assert!(false);
                }
            }

            fs::remove_file(path).unwrap();
        }
    }
}
