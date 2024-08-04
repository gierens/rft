use anyhow::{anyhow, Context};
use runtime_sized_array::Array;
use std::fmt::{self, Debug, Formatter};
use std::mem::size_of;
use zerocopy::{AsBytes, FromBytes};
use zerocopy_derive::{AsBytes, FromBytes, FromZeroes};

trait ParseZeroCopy<'a> {
    fn parse(bytes: &'a [u8], index: &mut usize) -> Result<&'a Self, anyhow::Error>
    where
        Self: Sized;
}

impl<'a, T> ParseZeroCopy<'a> for T
where
    T: FromBytes,
{
    fn parse(bytes: &'a [u8], index: &mut usize) -> Result<&'a Self, anyhow::Error>
    where
        Self: Sized,
    {
        let size = size_of::<Self>();
        let name = std::any::type_name::<Self>();
        if *index + size > bytes.len() {
            return Err(anyhow!("Buffer to short for {name}"));
        }
        let obj =
            Self::ref_from(&bytes[*index..*index + size]).context("Cannot reference as {name}")?;
        *index += size;
        Ok(obj)
    }
}

trait Parse<'a> {
    fn parse(bytes: &'a [u8], index: &mut usize) -> Result<Self, anyhow::Error>
    where
        Self: Sized;
}

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
    Error(&'a ErrorFrame<'a>),
    Data(&'a DataFrame<'a>),
    Read(&'a ReadCommand<'a>),
    Write(&'a WriteCommand<'a>),
    Checksum(&'a ChecksumCommand<'a>),
    Stat(&'a StatCommand<'a>),
    List(&'a ListCommand<'a>),
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
    pub payload_length: u16,
}

// TODO:
// - fork and stabilize runtime_sized_array
// - use runtime_sized_array for payload data in other structs
// - write macro to simplify trait impls

pub struct AnswerFrame<'a> {
    pub header: &'a AnswerHeader,
    pub payload: Array<u8>,
}

impl Debug for AnswerFrame<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let payload = self.payload.clone().into_vec();
        f.debug_struct("AnswerFrame")
            .field("header", &self.header)
            .field("payload", &payload)
            .finish()
    }
}

impl<'a> Parse<'a> for AnswerFrame<'a> {
    fn parse(bytes: &'a [u8], index: &mut usize) -> Result<AnswerFrame<'a>, anyhow::Error> {
        let name = std::any::type_name::<Self>();
        let header_size = size_of::<AnswerHeader>();
        let header = AnswerHeader::parse(&bytes, index)?;
        if *index + header.payload_length as usize > bytes.len() {
            return Err(anyhow!("Buffer to short for {name}"));
        }
        let payload = unsafe {
            Array::from_pointer(
                bytes[*index..*index + header.payload_length as usize]
                    .as_ptr()
                    .cast_mut(),
                header.payload_length as usize,
            )
        };
        *index += header.payload_length as usize;
        Ok(AnswerFrame { header, payload })
    }
}

impl AnswerFrame<'_> {
    pub fn as_vec(self) -> Vec<u8> {
        let mut vec1: Vec<u8> = self.header.as_bytes().into();
        let mut vec2: Vec<u8> = self.payload.into_vec().clone();
        vec1.append(&mut vec2);
        vec1
    }
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

impl<'a> Parse<'a> for Packet<'a> {
    fn parse(bytes: &'a [u8], index: &mut usize) -> Result<Packet<'a>, anyhow::Error> {
        let header = PacketHeader::parse(bytes, index)?;
        let mut packet = Packet {
            header,
            frames: Vec::new(),
        };
        while *index < bytes.len() {
            let frame = match bytes[*index] {
                0 => Ack(AckFrame::parse(bytes, index)?),
                1 => Exit(ExitFrame::parse(bytes, index)?),
                2 => ConnIdChange(ConnIdChangeFrame::parse(bytes, index)?),
                3 => FlowControl(FlowControlFrame::parse(bytes, index)?),
                4 => Answer(AnswerFrame::parse(bytes, index)?),
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

impl<'a> Packet<'a> {
    pub fn parse_full(bytes: &'a [u8]) -> Result<Packet<'a>, anyhow::Error> {
        let mut index = 0;
        let packet = Self::parse(bytes, &mut index)?;
        if index != bytes.len() {
            return Err(anyhow!("Buffer too long for Packet"));
        }
        Ok(packet)
    }
}
