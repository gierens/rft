use anyhow::anyhow;
use bytes::Bytes;
use std::fmt::Debug;
use std::mem::size_of;
use zerocopy::FromBytes;

use crate::protocol::*;

trait Parse {
    fn parse(bytes: &mut Bytes) -> Result<Frame, anyhow::Error>
    where
        Self: Sized;
}

pub struct Packet {
    header_bytes: Bytes,
    pub frames: Vec<Frame>,
}

impl Debug for Packet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Packet")
            .field("header", &self.header())
            .field("frames", &self.frames)
            .finish()
    }
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
pub enum Frames<'a> {
    Ack(&'a AckFrame),
    Exit(&'a ExitFrame),
    ConnIdChange(&'a ConnIdChangeFrame),
    FlowControl(&'a FlowControlFrame),
    Answer(AnswerFrame<'a>),
    // Error(&'a ErrorFrame<'a>),
    // Data(&'a DataFrame<'a>),
    // Read(&'a ReadCommand<'a>),
    // Write(&'a WriteCommand<'a>),
    // Checksum(&'a ChecksumCommand<'a>),
    // Stat(&'a StatCommand<'a>),
    // List(&'a ListCommand<'a>),
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

    pub fn header(&'a self) -> Frames<'a> {
        match self.code() {
            0 => Frames::Ack(self.into()),
            1 => Frames::Exit(self.into()),
            2 => Frames::ConnIdChange(self.into()),
            3 => Frames::FlowControl(self.into()),
            4 => Frames::Answer(self.into()),
            _ => panic!("Unknown frame type"),
        }
    }

    pub fn payload(&self) -> Option<&Bytes> {
        self.payload_bytes.as_ref()
    }
}

impl<'a> From<&'a Frame> for &'a AckFrame {
    fn from(frame: &'a Frame) -> Self {
        AckFrame::ref_from(frame.header_bytes.as_ref()).expect("Failed to reference AckFrame")
    }
}

impl Parse for AckFrame {
    fn parse(bytes: &mut Bytes) -> Result<Frame, anyhow::Error> {
        // TODO bounds check
        let header_bytes = bytes.split_to(size_of::<AckFrame>());
        Ok(Frame {
            header_bytes,
            payload_bytes: None,
        })
    }
}

impl<'a> From<&'a Frame> for &'a ExitFrame {
    fn from(frame: &'a Frame) -> Self {
        ExitFrame::ref_from(frame.header_bytes.as_ref()).expect("Failed to reference ExitFrame")
    }
}

impl Parse for ExitFrame {
    fn parse(bytes: &mut Bytes) -> Result<Frame, anyhow::Error> {
        // TODO bounds check
        let header_bytes = bytes.split_to(size_of::<ExitFrame>());
        Ok(Frame {
            header_bytes,
            payload_bytes: None,
        })
    }
}

impl<'a> From<&'a Frame> for &'a ConnIdChangeFrame {
    fn from(frame: &'a Frame) -> Self {
        ConnIdChangeFrame::ref_from(frame.header_bytes.as_ref())
            .expect("Failed to reference ConnIdChangeFrame")
    }
}

impl Parse for ConnIdChangeFrame {
    fn parse(bytes: &mut Bytes) -> Result<Frame, anyhow::Error> {
        // TODO bounds check
        let header_bytes = bytes.split_to(size_of::<ConnIdChangeFrame>());
        Ok(Frame {
            header_bytes,
            payload_bytes: None,
        })
    }
}

impl<'a> From<&'a Frame> for &'a FlowControlFrame {
    fn from(frame: &'a Frame) -> Self {
        FlowControlFrame::ref_from(frame.header_bytes.as_ref())
            .expect("Failed to reference FlowControlFrame")
    }
}

impl Parse for FlowControlFrame {
    fn parse(bytes: &mut Bytes) -> Result<Frame, anyhow::Error> {
        // TODO bounds check
        let header_bytes = bytes.split_to(size_of::<FlowControlFrame>());
        Ok(Frame {
            header_bytes,
            payload_bytes: None,
        })
    }
}

#[derive(Debug)]
pub struct AnswerFrame<'a> {
    pub header: &'a AnswerHeader,
    pub payload: &'a Bytes,
}

impl<'a> From<&'a Frame> for AnswerFrame<'a> {
    fn from(frame: &'a Frame) -> Self {
        AnswerFrame {
            header: AnswerHeader::ref_from(frame.header_bytes.as_ref())
                .expect("Failed to reference AnswerFrame"),
            payload: frame.payload().expect("Missing payload in AnswerFrame"),
        }
    }
}

impl<'a> Parse for AnswerFrame<'a> {
    fn parse(bytes: &mut Bytes) -> Result<Frame, anyhow::Error> {
        // TODO bounds check
        let header_bytes = bytes.split_to(size_of::<AnswerHeader>());
        let length_bytes = bytes.split_to(2);
        let payload_length = length_bytes[0] as usize | (length_bytes[1] as usize) << 8;
        let payload_bytes = bytes.split_to(payload_length);
        Ok(Frame {
            header_bytes,
            payload_bytes: Some(payload_bytes),
        })
    }
}
