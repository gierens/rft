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
    ConnIdChange(&'a ConnIdChangeFrame),
    FlowControl(&'a FlowControlFrame),
    Answer(AnswerFrame<'a>),
    Error(ErrorFrame<'a>),
    Data(DataFrame<'a>),
    Read(ReadCommand<'a>),
    Write(WriteCommand<'a>),
    Checksum(ChecksumCommand<'a>),
    Stat(StatCommand<'a>),
    List(ListCommand<'a>),
}
use Frame::*;

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

macro_rules! frame_from_bytes {
    ($bytes:ident, $index:ident, $variant:ident, $frame:ident) => {{
        let frame_size = size_of::<$frame>();
        if $bytes.len() - $index < frame_size {
            return Err(anyhow!("Buffer to short for $frame"));
        }
        let frame = $variant(
            $frame::ref_from(&$bytes[$index..$index + frame_size])
                .context("Cannot transmute $frame")?,
        );
        $index += frame_size;
        frame
    }};
}

impl<'a> Packet<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Packet, anyhow::Error> {
        let header_size = size_of::<PacketHeader>();
        if bytes.len() < header_size {
            return Err(anyhow!("Buffer to short for packet header"));
        }
        let mut packet = Packet {
            header: PacketHeader::ref_from(&bytes[0..header_size])
                .context("Cannot transmute packet header")?,
            frames: Vec::new(),
        };
        let mut index = header_size;
        while index < bytes.len() {
            let frame = match bytes[index] {
                0 => frame_from_bytes!(bytes, index, Ack, AckFrame),
                1 => frame_from_bytes!(bytes, index, Exit, ExitFrame),
                2 => frame_from_bytes!(bytes, index, ConnIdChange, ConnIdChangeFrame),
                3 => frame_from_bytes!(bytes, index, FlowControl, FlowControlFrame),
                // 4 => frame_from_bytes!(bytes, index, Answer, AnswerFrame),
                // 5 => frame_from_bytes!(bytes, index, Error, ErrorFrame),
                // 6 => frame_from_bytes!(bytes, index, Data, DataFrame),
                // 7 => frame_from_bytes!(bytes, index, Read, ReadCommand),
                // 8 => frame_from_bytes!(bytes, index, Write, WriteCommand),
                // 9 => frame_from_bytes!(bytes, index, Checksum, ChecksumCommand),
                // 10 => frame_from_bytes!(bytes, index, Stat, StatCommand),
                // 11 => frame_from_bytes!(bytes, index, List, ListCommand),
                _ => continue,
            };
            packet.frames.push(frame);
        }
        Ok(packet)
    }
}
