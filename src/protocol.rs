use anyhow::{anyhow, Context};
use bytes::Bytes;
use runtime_sized_array::Array;
use std::fmt::{self, Debug, Formatter};
use std::mem::size_of;
use zerocopy::{AsBytes, FromBytes};
use zerocopy_derive::{AsBytes, FromBytes, FromZeroes};

#[derive(Debug)]
pub struct PacketParser {
    pub packet: Packet,
}

impl PacketParser {
    pub fn parse(bytes: Bytes) -> Result<PacketParser, anyhow::Error> {
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
                    let mut payload_bytes = header_bytes.split_off(size_of::<AnswerFrame>());
                    let payload_length =
                        payload_bytes[0] as usize | (payload_bytes[1] as usize) << 8;
                    frame_bytes = payload_bytes.split_off(2 + payload_length);
                    packet.frames.push(Frame {
                        header_bytes,
                        payload_bytes: Some(payload_bytes),
                    });
                }
                _ => return Err(anyhow!("Unknown frame type")),
            }
        }
        Ok(PacketParser { packet })
    }
}

#[derive(Debug, AsBytes, FromZeroes, FromBytes)]
#[repr(C, packed)]
pub struct PacketHeader {
    pub version: u8,
    pub connection_id: u32,
    pub checksum: [u8; 3],
}

#[derive(Debug)]
pub struct Packet {
    header_bytes: Bytes,
    pub frames: Vec<Frame>,
}

impl Packet {
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

    fn answer(&self) -> &AnswerFrame {
        AnswerFrame::ref_from(self.header_bytes.as_ref()).expect("Failed to parse AnswerFrame")
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
pub enum FrameHeader<'a> {
    Ack(&'a AckFrame),
    Exit(&'a ExitFrame),
    ConnIdChange(&'a ConnIdChangeFrame),
    FlowControl(&'a FlowControlFrame),
    Answer(&'a AnswerFrame),
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

#[derive(Debug, AsBytes, FromZeroes, FromBytes)]
#[repr(C, packed)]
pub struct AckFrame {
    pub typ: u8,
    pub stream_id: u16,
    pub frame_id: u32,
}

#[derive(Debug, AsBytes, FromZeroes, FromBytes)]
#[repr(C, packed)]
pub struct ExitFrame {
    pub typ: u8,
}

#[derive(Debug, AsBytes, FromZeroes, FromBytes)]
#[repr(C, packed)]
pub struct ConnIdChangeFrame {
    pub typ: u8,
    pub old_cid: u32,
    pub new_cid: u32,
}

#[derive(Debug, AsBytes, FromZeroes, FromBytes)]
#[repr(C, packed)]
pub struct FlowControlFrame {
    pub typ: u8,
    pub window_size: u32,
}

#[derive(Debug, AsBytes, FromZeroes, FromBytes)]
#[repr(C, packed)]
pub struct AnswerFrame {
    pub typ: u8,
    pub stream_id: u16,
    pub frame_id: u32,
    pub command_frame_id: u32,
}

// pub struct AnswerFrame<'a> {
//     pub header: &'a AnswerHeader,
//     pub payload: Array<u8>,
// }
//
// #[derive(Debug, AsBytes, FromZeroes, FromBytes)]
// #[repr(C, packed)]
// pub struct ErrorFrameHeader {
//     pub typ: u8,
//     pub stream_id: u16,
//     pub frame_id: u32,
//     pub command_frame_id: u32,
// }
//
// #[derive(Debug)]
// pub struct ErrorFrame<'a> {
//     pub header: ErrorFrameHeader,
//     pub payload: &'a [u8],
// }
//
// #[derive(Debug, AsBytes, FromZeroes, FromBytes)]
// #[repr(C, packed)]
// pub struct DataHeader {
//     pub typ: u8,
//     pub stream_id: u16,
//     pub frame_id: u32,
//     pub offset: [u8; 3],
//     pub length: [u8; 3],
// }
//
// #[derive(Debug)]
// pub struct DataFrame<'a> {
//     pub header: DataHeader,
//     pub payload: &'a [u8],
// }
//
// #[derive(Debug, AsBytes, FromZeroes, FromBytes)]
// #[repr(C, packed)]
// pub struct ReadHeader {
//     pub typ: u8,
//     pub stream_id: u16,
//     pub frame_id: u32,
//     pub flags: u8,
//     pub offset: [u8; 3],
//     pub length: [u8; 3],
//     pub checksum: u32,
// }
//
// #[derive(Debug)]
// pub struct ReadCommand<'a> {
//     pub header: ReadHeader,
//     pub path: &'a str,
// }
//
// #[derive(Debug, AsBytes, FromZeroes, FromBytes)]
// #[repr(C, packed)]
// pub struct ChecksumHeader {
//     pub typ: u8,
//     pub stream_id: u16,
//     pub frame_id: u32,
// }
//
// #[derive(Debug, AsBytes, FromZeroes, FromBytes)]
// #[repr(C, packed)]
// pub struct WriteHeader {
//     pub typ: u8,
//     pub stream_id: u16,
//     pub frame_id: u32,
//     pub offset: [u8; 3],
//     pub length: [u8; 3],
// }
//
// #[derive(Debug)]
// pub struct WriteCommand<'a> {
//     pub header: WriteHeader,
//     pub path: &'a str,
// }
//
// #[derive(Debug)]
// pub struct ChecksumCommand<'a> {
//     pub header: ChecksumHeader,
//     pub path: &'a str,
// }
//
// #[derive(Debug, AsBytes, FromZeroes, FromBytes)]
// #[repr(C, packed)]
// pub struct StatHeader {
//     pub typ: u8,
//     pub stream_id: u16,
//     pub frame_id: u32,
// }
//
// #[derive(Debug)]
// pub struct StatCommand<'a> {
//     pub header: StatHeader,
//     pub path: &'a str,
// }
//
// #[derive(Debug, AsBytes, FromZeroes, FromBytes)]
// #[repr(C, packed)]
// pub struct ListHeader {
//     pub typ: u8,
//     pub stream_id: u16,
//     pub frame_id: u32,
// }
//
// #[derive(Debug)]
// pub struct ListCommand<'a> {
//     pub header: ListHeader,
//     pub path: &'a str,
// }
