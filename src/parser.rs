use anyhow::anyhow;
use bytes::Bytes;
use std::fmt::Debug;
use std::mem::size_of;
use zerocopy::FromBytes;

use crate::protocol::*;

#[derive(Debug)]
pub struct Packet {
    header_bytes: Bytes,
    pub frames: Vec<Frame>,
}

impl Packet {
    pub fn parse(bytes: Bytes) -> Result<Self, anyhow::Error> {
        let mut header_bytes = bytes;
        let mut frame_bytes = header_bytes.split_off(size_of::<PacketHeader>());
        let mut packet = Packet {
            header_bytes,
            frames: Vec::new(),
        };
        while !frame_bytes.is_empty() {
            let mut header_bytes = frame_bytes;
            let code = header_bytes[0];
            dbg!(code);
            match code {
                0 => {
                    frame_bytes = header_bytes.split_off(size_of::<AckFrame>());
                    packet.frames.push(Frame {
                        header_bytes,
                        payload_bytes: None,
                    });
                }
                1 => {
                    frame_bytes = header_bytes.split_off(size_of::<ExitFrame>());
                    packet.frames.push(Frame {
                        header_bytes,
                        payload_bytes: None,
                    });
                }
                2 => {
                    frame_bytes = header_bytes.split_off(size_of::<ConnIdChangeFrame>());
                    packet.frames.push(Frame {
                        header_bytes,
                        payload_bytes: None,
                    });
                }
                3 => {
                    frame_bytes = header_bytes.split_off(size_of::<FlowControlFrame>());
                    packet.frames.push(Frame {
                        header_bytes,
                        payload_bytes: None,
                    });
                }
                4 => {
                    let mut payload_bytes = header_bytes.split_off(size_of::<AnswerHeader>());
                    let payload_length =
                        payload_bytes[0] as usize | (payload_bytes[1] as usize) << 8;
                    payload_bytes = payload_bytes.split_off(2);
                    frame_bytes = payload_bytes.split_off(payload_length);
                    packet.frames.push(Frame {
                        header_bytes,
                        payload_bytes: Some(payload_bytes),
                    });
                }
                _ => return Err(anyhow!("Unknown frame type")),
            }
        }
        Ok(packet)
    }

    pub fn header(&self) -> &PacketHeader {
        PacketHeader::ref_from(self.header_bytes.as_ref()).expect("Failed to parse PacketHeader")
    }
}

#[derive(Debug)]
pub struct Frame {
    header_bytes: Bytes,
    payload_bytes: Option<Bytes>,
}

impl<'a> Frame {
    fn code(&self) -> u8 {
        self.header_bytes[0]
    }

    fn ack(&self) -> &AckFrame {
        AckFrame::ref_from(self.header_bytes.as_ref()).expect("Failed to parse AckFrame")
    }

    fn exit(&self) -> &ExitFrame {
        ExitFrame::ref_from(self.header_bytes.as_ref()).expect("Failed to parse ExitFrame")
    }

    fn conn_id_change(&self) -> &ConnIdChangeFrame {
        ConnIdChangeFrame::ref_from(self.header_bytes.as_ref())
            .expect("Failed to parse ConnIdChangeFrame")
    }

    fn flow_control(&self) -> &FlowControlFrame {
        FlowControlFrame::ref_from(self.header_bytes.as_ref())
            .expect("Failed to parse FlowControlFrame")
    }

    fn answer(&self) -> AnswerFrame {
        AnswerFrame {
            header: AnswerHeader::ref_from(self.header_bytes.as_ref())
                .expect("Failed to parse AnswerFrame"),
            payload: self
                .payload_bytes
                .as_ref()
                .expect("Missing payload in AnswerFrame"),
        }
    }

    pub fn header(&'a self) -> FrameHeader<'a> {
        match self.code() {
            0 => FrameHeader::Ack(self.ack()),
            1 => FrameHeader::Exit(self.exit()),
            2 => FrameHeader::ConnIdChange(self.conn_id_change()),
            3 => FrameHeader::FlowControl(self.flow_control()),
            4 => FrameHeader::Answer(self.answer()),
            _ => panic!("Unknown frame type"),
        }
    }

    pub fn payload(&self) -> Option<&Bytes> {
        self.payload_bytes.as_ref()
    }
}

#[derive(Debug)]
pub struct AnswerFrame<'a> {
    pub header: &'a AnswerHeader,
    pub payload: &'a Bytes,
}

#[derive(Debug)]
pub enum FrameHeader<'a> {
    Ack(&'a AckFrame),
    Exit(&'a ExitFrame),
    ConnIdChange(&'a ConnIdChangeFrame),
    FlowControl(&'a FlowControlFrame),
    Answer(&'a AnswerHeader),
    // Error(&'a ErrorFrame<'a>),
    // Data(&'a DataFrame<'a>),
    // Read(&'a ReadCommand<'a>),
    // Write(&'a WriteCommand<'a>),
    // Checksum(&'a ChecksumCommand<'a>),
    // Stat(&'a StatCommand<'a>),
    // List(&'a ListCommand<'a>),
}

// #[derive(Debug)]
// pub enum Frame<'a> {
//     Ack(&'a AckFrame),
//     Exit(&'a ExitFrame),
//     ConnIdChange(&'a ConnIdChangeFrame),
//     FlowControl(&'a FlowControlFrame),
//     Answer(AnswerFrame<'a>),
//     Error(&'a ErrorFrame<'a>),
//     Data(&'a DataFrame<'a>),
//     Read(&'a ReadCommand<'a>),
//     Write(&'a WriteCommand<'a>),
//     Checksum(&'a ChecksumCommand<'a>),
//     Stat(&'a StatCommand<'a>),
//     List(&'a ListCommand<'a>),
// }
// use Frame::*;
