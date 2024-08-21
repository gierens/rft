use crate::wire::{AnswerFrame, DataFrame, ErrorFrame, Frame};
use anyhow::{anyhow, Result};
use bytes::Bytes;
use futures::{Sink, SinkExt, Stream, StreamExt};
use std::cmp::min;
use std::fmt::Debug;
use std::fs;
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
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
                    //parse path
                    let path: String = match cmd.path().to_str() {
                        Some(s) => s.into(),
                        None => {
                            sink.send(ErrorFrame::new(cmd.stream_id(), "Invalid Payload").into())
                                .await
                                .expect("stream_handler: could not send response");
                            return Ok(());
                        }
                    };

                    //open file
                    let file: File = match OpenOptions::new().read(true).open(path.clone()) {
                        Ok(f) => f,
                        Err(e) => {
                            sink.send(
                                ErrorFrame::new(cmd.stream_id(), e.to_string().as_str()).into(),
                            )
                            .await
                            .expect("stream_handler: could not send response");
                            return Ok(());
                        }
                    };

                    //get file size
                    let metadata = fs::metadata(path.clone()).expect("Could not get file metadata");
                    let file_size = metadata.len();

                    //check if trying to read past EOF
                    if cmd.offset() + cmd.length() > file_size {
                        sink.send(
                            ErrorFrame::new(cmd.stream_id(), "You're trying to read past EOF")
                                .into(),
                        )
                        .await
                        .expect("stream_handler: could not send response");
                        return Ok(());
                    }

                    let read_target = match cmd.length() {
                        0 => file_size,
                        _ => min(cmd.offset() + cmd.length(), file_size),
                    };

                    //move cursor to offset
                    //TODO: may have to check manually if offset is past file size ??
                    let mut reader = BufReader::new(file);
                    match reader.seek(SeekFrom::Start(cmd.offset())) {
                        Ok(_) => {}
                        Err(e) => {
                            sink.send(
                                ErrorFrame::new(cmd.stream_id(), e.to_string().as_str()).into(),
                            )
                            .await
                            .expect("stream_handler: could not send response");
                            return Ok(());
                        }
                    }

                    //read data from file and generate data frames
                    let mut last_offset = cmd.offset(); //the first byte not yet sent
                    let mut fin = false;
                    let mut read_buf = [0u8; 128]; //TODO: which buf size to use? 128 for tests.
                    loop {
                        //check if we are finished
                        if last_offset >= read_target && fin {
                            break;
                        };

                        //read bytes from file into buf
                        let mut data_size = reader.read(&mut read_buf).expect("file read error");

                        //check if we reached read_target -> this frame is EOF
                        if last_offset >= read_target {
                            data_size = 0;
                            fin = true;
                        }

                        //check if we read past read_target in this iteration
                        if last_offset + (data_size as u64) >= read_target {
                            //adjust data_size to only send data up to read_target
                            data_size -=
                                ((last_offset + (data_size as u64)) - read_target) as usize;
                        }

                        let data_bytes = Bytes::copy_from_slice(&read_buf[..data_size]);

                        //assemble and dispatch data frame
                        {
                            sink.send(
                                DataFrame::new(cmd.stream_id(), last_offset, data_bytes).into(),
                            )
                            .await
                            .expect("stream_handler: could not send response");
                        }

                        //update counters
                        last_offset += data_size as u64;
                    }

                    Ok(())
                }

                Frame::Write(cmd) => {
                    //parse path
                    let path: String = match cmd.path().to_str() {
                        Some(s) => s.into(),
                        None => {
                            sink.send(ErrorFrame::new(cmd.stream_id(), "Invalid Payload").into())
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
                            sink.send(
                                ErrorFrame::new(cmd.stream_id(), e.to_string().as_str()).into(),
                            )
                            .await
                            .expect("stream_handler: could not send response");
                            return Ok(());
                        }
                    };

                    //check if file size matches write offset
                    let metadata = fs::metadata(path.clone()).expect("Could not get file metadata");
                    if metadata.len() != cmd.offset() {
                        sink.send(
                            ErrorFrame::new(
                                cmd.stream_id(),
                                "Write offset does not match file size",
                            )
                            .into(),
                        )
                        .await
                        .expect("stream_handler: could not send response");
                        return Ok(());
                    }

                    //receive Data frames and write to file; stop if transmission complete
                    let mut writer = BufWriter::new(file);
                    let mut last_offset = cmd.offset();
                    loop {
                        let next_frame = match timeout(Duration::from_secs(5), stream.next()).await
                        {
                            Ok(f) => f,
                            Err(_) => {
                                //timeout: sed error frame, exit
                                sink.send(ErrorFrame::new(cmd.stream_id(), "Timeout").into())
                                    .await
                                    .expect("stream_handler: could not send response");
                                return Ok(());
                            }
                        };

                        if let Some(Frame::Data(f)) = next_frame {
                            //empty data frame marks end of transmission
                            if f.length() == 0 {
                                break;
                            }

                            //check if offset matches
                            if last_offset != f.offset() {
                                //mismatch -> send Error Frame, abort
                                sink.send(
                                    ErrorFrame::new(
                                        cmd.stream_id(),
                                        "Write offset mismatch, aborting...",
                                    )
                                    .into(),
                                )
                                .await
                                .expect("stream_handler: could not send Error");
                                break;
                            }

                            //write data from frame to file
                            writer
                                .write_all(f.payload())
                                .expect("Could not write to BufWriter");

                            //update last received frame id and offset
                            last_offset += f.length();
                        } else {
                            //illegal frame or channel closed: abort transmission and leave file so client can continue later
                            sink.send(
                                ErrorFrame::new(cmd.stream_id(), "Illegal Frame Received").into(),
                            )
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
                                        Bytes::copy_from_slice(digest.as_ref()),
                                    )
                                    .into(),
                                )
                                .await
                                .expect("stream_handler: could not send response");
                            }
                            Err(e) => {
                                sink.send(
                                    ErrorFrame::new(cmd.stream_id(), e.to_string().as_str()).into(),
                                )
                                .await
                                .expect("stream_handler: could not send response");
                                return Ok(());
                            }
                        },
                        None => {
                            sink.send(ErrorFrame::new(cmd.stream_id(), "Invalid Payload").into())
                                .await
                                .expect("stream_handler: could not send response");
                            return Ok(());
                        }
                    }

                    Ok(())
                }

                Frame::Stat(cmd) => {
                    sink.send(ErrorFrame::new(cmd.stream_id(), "Not implemented").into())
                        .await
                        .expect("stream_handler: could not send response");
                    Ok(())
                }

                Frame::List(cmd) => {
                    sink.send(ErrorFrame::new(cmd.stream_id(), "Not implemented").into())
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
    use crate::wire::Frame::Error;
    use crate::wire::{ChecksumFrame, DataFrame, ReadFrame, WriteFrame};
    use crate::wire::{Frame, Frame::Answer};
    use data_encoding::HEXLOWER;
    use futures::channel::mpsc::{channel, Receiver, Sender};
    use std::path::Path;
    use std::str;

    #[tokio::test]
    async fn test_checksum() {
        let path = "testfile.txt";
        let mut out = File::create(path).unwrap();
        write!(out, "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.").unwrap();
        {
            let (mut itx, irx): (Sender<Frame>, Receiver<Frame>) = channel(1);
            let (otx, mut orx): (Sender<Frame>, Receiver<Frame>) = channel(1);
            itx.send(ChecksumFrame::new(420, Path::new(path)).into())
                .await
                .unwrap();

            match stream_handler(irx, otx).await {
                Ok(()) => {
                    let af = orx.next().await.unwrap();

                    match af {
                        Answer(a) => {
                            assert_eq!(a.type_id(), 4);
                            let sid = a.stream_id();
                            assert_eq!(sid, 420);

                            let s = HEXLOWER.encode(a.payload());

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

        {
            let (mut itx, irx): (Sender<Frame>, Receiver<Frame>) = channel(1);
            let (otx, mut orx): (Sender<Frame>, Receiver<Frame>) = channel(1);
            itx.send(ChecksumFrame::new(420, &Path::new(path)).into())
                .await
                .unwrap();

            match stream_handler(irx, otx).await {
                Ok(()) => {
                    let af = orx.next().await.unwrap();

                    match af {
                        Error(e) => {
                            assert_eq!(e.message(), "No such file or directory (os error 2)");
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

        let dp1_bytes = Bytes::copy_from_slice(&payload.as_bytes()[..128]);
        let dp2_bytes = Bytes::copy_from_slice(&payload.as_bytes()[128..]);
        let dp3_bytes = Bytes::default();

        let stream_id = 420;

        {
            let (mut itx, irx): (Sender<Frame>, Receiver<Frame>) = channel(5);
            let (otx, _orx): (Sender<Frame>, Receiver<Frame>) = channel(5);

            //send command frame
            itx.send(WriteFrame::new(stream_id, 0, 334, Path::new(path)).into())
                .await
                .unwrap();

            //send data frames
            itx.send(DataFrame::new(stream_id, 0, dp1_bytes).into())
                .await
                .unwrap();

            itx.send(DataFrame::new(stream_id, 128, dp2_bytes).into())
                .await
                .unwrap();

            //send EOF frame
            itx.send(DataFrame::new(stream_id, 334 - 128, dp3_bytes).into())
                .await
                .unwrap();

            //run handler and test whether file written
            match stream_handler(irx, otx).await {
                Ok(()) => {
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

        {
            let (mut itx, irx): (Sender<Frame>, Receiver<Frame>) = channel(4);
            let (otx, mut orx): (Sender<Frame>, Receiver<Frame>) = channel(5);

            //send read command
            itx.send(ReadFrame::new(69, 0, 0, 0, 0, Path::new(path)).into())
                .await
                .unwrap();

            let mut rec = String::new();

            //start handler
            match stream_handler(irx, otx).await {
                Ok(_) => {
                    //receive three data frames + EOF, check whether contents are correct

                    let fh1 = orx.next().await.unwrap();

                    match fh1 {
                        Frame::Data(d) => rec.push_str(str::from_utf8(d.payload()).unwrap()),
                        _ => {
                            assert!(false)
                        }
                    }

                    let fh2 = orx.next().await.unwrap();

                    match fh2 {
                        Frame::Data(d) => rec.push_str(str::from_utf8(d.payload()).unwrap()),
                        _ => {
                            assert!(false)
                        }
                    }

                    let fh3 = orx.next().await.unwrap();

                    match fh3 {
                        Frame::Data(d) => rec.push_str(str::from_utf8(d.payload()).unwrap()),
                        _ => {
                            assert!(false)
                        }
                    }

                    //EOF
                    let fh4 = orx.next().await.unwrap();

                    match fh4 {
                        Frame::Data(d) => {
                            assert_eq!(d.length(), 0);
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
