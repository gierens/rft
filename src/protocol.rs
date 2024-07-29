use anyhow::{anyhow, Context};
use std::mem::size_of;
use zerocopy::FromBytes;
use zerocopy_derive::{AsBytes, FromBytes, FromZeroes};

#[derive(Debug, AsBytes, FromZeroes, FromBytes)]
#[repr(C, packed)]
pub struct PacketHeader {
    pub version: u8,
    pub connection_id: u32,
    pub checksum: [u8; 3],
}

#[derive(Debug)]
pub struct Packet<'a> {
    pub header: &'a PacketHeader,
    pub frames: Vec<Frame<'a>>,
}

#[derive(Debug)]
pub enum Frame<'a> {
    Ack(&'a AckFrame),
    Exit(&'a ExitFrame),
}

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
pub struct AnswerHeader {
    pub typ: u8,
    pub stream_id: u16,
    pub frame_id: u32,
    pub command_frame_id: u32,
}

#[derive(Debug)]
pub struct AnswerFrame<'a> {
    pub header: AnswerHeader,
    pub payload: &'a [u8],
}

#[derive(Debug, AsBytes, FromZeroes, FromBytes)]
#[repr(C, packed)]
pub struct ErrorFrameHeader {
    pub typ: u8,
    pub stream_id: u16,
    pub frame_id: u32,
    pub command_frame_id: u32,
}

#[derive(Debug)]
pub struct ErrorFrame<'a> {
    pub header: ErrorFrameHeader,
    pub payload: &'a [u8],
}

#[derive(Debug, AsBytes, FromZeroes, FromBytes)]
#[repr(C, packed)]
pub struct DataHeader {
    pub typ: u8,
    pub stream_id: u16,
    pub frame_id: u32,
    pub offset: [u8; 3],
    pub length: [u8; 3],
}

#[derive(Debug)]
pub struct DataFrame<'a> {
    pub header: DataHeader,
    pub payload: &'a [u8],
}

#[derive(Debug, AsBytes, FromZeroes, FromBytes)]
#[repr(C, packed)]
pub struct ReadHeader {
    pub typ: u8,
    pub stream_id: u16,
    pub frame_id: u32,
    pub flags: u8,
    pub offset: [u8; 3],
    pub length: [u8; 3],
    pub checksum: u32,
}

#[derive(Debug)]
pub struct ReadCommand<'a> {
    pub header: ReadHeader,
    pub path: &'a str,
}

#[derive(Debug, AsBytes, FromZeroes, FromBytes)]
#[repr(C, packed)]
pub struct ChecksumHeader {
    pub typ: u8,
    pub stream_id: u16,
    pub frame_id: u32,
}

#[derive(Debug, AsBytes, FromZeroes, FromBytes)]
#[repr(C, packed)]
pub struct WriteHeader {
    pub typ: u8,
    pub stream_id: u16,
    pub frame_id: u32,
    pub offset: [u8; 3],
    pub length: [u8; 3],
}

#[derive(Debug)]
pub struct WriteCommand<'a> {
    pub header: WriteHeader,
    pub path: &'a str,
}

#[derive(Debug)]
pub struct ChecksumCommand<'a> {
    pub header: ChecksumHeader,
    pub path: &'a str,
}

#[derive(Debug, AsBytes, FromZeroes, FromBytes)]
#[repr(C, packed)]
pub struct StatHeader {
    pub typ: u8,
    pub stream_id: u16,
    pub frame_id: u32,
}

#[derive(Debug)]
pub struct StatCommand<'a> {
    pub header: StatHeader,
    pub path: &'a str,
}

#[derive(Debug, AsBytes, FromZeroes, FromBytes)]
#[repr(C, packed)]
pub struct ListHeader {
    pub typ: u8,
    pub stream_id: u16,
    pub frame_id: u32,
}

#[derive(Debug)]
pub struct ListCommand<'a> {
    pub header: ListHeader,
    pub path: &'a str,
}

impl<'a> Packet<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Packet, anyhow::Error> {
        let header_size = size_of::<PacketHeader>();
        if bytes.len() < header_size {
            return Err(anyhow!("Buffer to short for packet header"));
        }
        let mut packet = Packet {
            header: PacketHeader::ref_from(bytes).context("Cannot transmute packet header")?,
            frames: Vec::new(),
        };
        let index = header_size;
        while index < bytes.len() {
            let frame = match bytes[index] {
                0 => {
                    let frame_size = size_of::<AckFrame>();
                    if bytes.len() - index < frame_size {
                        return Err(anyhow!("Buffer to short for ack frame"));
                    }
                    Frame::Ack(
                        AckFrame::ref_from(&bytes[index..index + frame_size])
                            .context("Cannot transmute ack frame")?,
                    )
                }
                1 => {
                    let frame_size = size_of::<ExitFrame>();
                    if bytes.len() - index < frame_size {
                        return Err(anyhow!("Buffer to short for exit frame"));
                    }
                    Frame::Exit(
                        ExitFrame::ref_from(&bytes[index..index + frame_size])
                            .context("Cannot transmute exit frame")?,
                    )
                }
                _ => continue,
            };
            packet.frames.push(frame);
        }
        Ok(packet)
    }
}
