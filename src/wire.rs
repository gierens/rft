use anyhow::anyhow;
use bytes::{Bytes, BytesMut};
use std::mem::size_of;
use std::str::from_utf8;
use std::{fmt::Debug, path::Path};
use zerocopy::{AsBytes, FromBytes, FromZeroes};

const VERSION: u8 = 1;

pub trait Parse {
    fn parse(bytes: &mut Bytes) -> Result<Frame, anyhow::Error>
    where
        Self: Sized;
}

pub trait Assemble {
    fn assemble(&self) -> BytesMut;
}

pub trait Size {
    fn size(&self) -> usize;
}

#[derive(Debug, AsBytes, FromZeroes, FromBytes)]
#[repr(C, packed)]
pub struct PacketHeader {
    pub version: u8,
    pub connection_id: u32,
    pub packet_id: u32,
    pub checksum: [u8; 3],
}

impl PacketHeader {
    pub fn checksum(&self) -> u32 {
        self.checksum[0] as u32 | (self.checksum[1] as u32) << 8 | (self.checksum[2] as u32) << 16
    }
}

#[derive(Debug, AsBytes, FromZeroes, FromBytes)]
#[repr(C, packed)]
pub struct AckHeader {
    pub type_id: u8,
    pub packet_id: u32,
}

#[derive(Clone)]
pub struct AckFrame {
    bytes: Bytes,
}

impl Size for AckFrame {
    #[inline(always)]
    fn size(&self) -> usize {
        size_of::<AckHeader>()
    }
}

impl AckFrame {
    const TYPE_ID: u8 = 0;

    pub fn new(packet_id: u32) -> Self {
        let header = AckHeader {
            type_id: Self::TYPE_ID,
            packet_id,
        };
        let bytes = BytesMut::from(AsBytes::as_bytes(&header)).into();
        AckFrame { bytes }
    }

    pub fn header(&self) -> &AckHeader {
        AckHeader::ref_from(self.bytes.as_ref()).expect("Failed to reference AckHeader")
    }

    pub fn type_id(&self) -> u8 {
        self.header().type_id
    }

    pub fn packet_id(&self) -> u32 {
        self.header().packet_id
    }
}

impl Parse for AckFrame {
    fn parse(bytes: &mut Bytes) -> Result<Frame, anyhow::Error> {
        // TODO bounds check
        let bytes = bytes.split_to(size_of::<AckHeader>());
        Ok(AckFrame { bytes }.into())
    }
}

impl Assemble for AckFrame {
    fn assemble(&self) -> BytesMut {
        self.bytes.clone().into()
    }
}

impl Debug for AckFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Ack")
            .field("packet_id", &self.packet_id())
            .finish()
    }
}

#[derive(Debug, AsBytes, FromZeroes, FromBytes)]
#[repr(C, packed)]
pub struct ExitHeader {
    pub type_id: u8,
}

#[derive(Clone)]
pub struct ExitFrame {
    bytes: Bytes,
}

impl Size for ExitFrame {
    #[inline(always)]
    fn size(&self) -> usize {
        size_of::<ExitHeader>()
    }
}

impl ExitFrame {
    const TYPE_ID: u8 = 1;

    pub fn new() -> Self {
        let header = ExitHeader {
            type_id: Self::TYPE_ID,
        };
        let bytes = BytesMut::from(AsBytes::as_bytes(&header)).into();
        ExitFrame { bytes }
    }

    pub fn header(&self) -> &ExitHeader {
        ExitHeader::ref_from(self.bytes.as_ref()).expect("Failed to reference ExitHeader")
    }

    pub fn type_id(&self) -> u8 {
        self.header().type_id
    }
}

impl Default for ExitFrame {
    fn default() -> Self {
        Self::new()
    }
}

impl Parse for ExitFrame {
    fn parse(bytes: &mut Bytes) -> Result<Frame, anyhow::Error> {
        // TODO bounds check
        let bytes = bytes.split_to(size_of::<ExitHeader>());
        Ok(ExitFrame { bytes }.into())
    }
}

impl Assemble for ExitFrame {
    fn assemble(&self) -> BytesMut {
        self.bytes.clone().into()
    }
}

impl Debug for ExitFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Exit").finish()
    }
}

#[derive(Debug, AsBytes, FromZeroes, FromBytes)]
#[repr(C, packed)]
pub struct ConnIdChangeHeader {
    pub type_id: u8,
    pub old_cid: u32,
    pub new_cid: u32,
}

#[derive(Clone)]
pub struct ConnIdChangeFrame {
    bytes: Bytes,
}

impl Size for ConnIdChangeFrame {
    #[inline(always)]
    fn size(&self) -> usize {
        size_of::<ConnIdChangeHeader>()
    }
}

impl ConnIdChangeFrame {
    const TYPE_ID: u8 = 2;

    pub fn new(old_cid: u32, new_cid: u32) -> Self {
        let header = ConnIdChangeHeader {
            type_id: Self::TYPE_ID,
            old_cid,
            new_cid,
        };
        let bytes = BytesMut::from(AsBytes::as_bytes(&header)).into();
        ConnIdChangeFrame { bytes }
    }

    pub fn header(&self) -> &ConnIdChangeHeader {
        ConnIdChangeHeader::ref_from(self.bytes.as_ref())
            .expect("Failed to reference ConnIdChangeHeader")
    }

    pub fn type_id(&self) -> u8 {
        self.header().type_id
    }

    pub fn old_cid(&self) -> u32 {
        self.header().old_cid
    }

    pub fn new_cid(&self) -> u32 {
        self.header().new_cid
    }
}

impl Parse for ConnIdChangeFrame {
    fn parse(bytes: &mut Bytes) -> Result<Frame, anyhow::Error> {
        // TODO bounds check
        let bytes = bytes.split_to(size_of::<ConnIdChangeHeader>());
        Ok(ConnIdChangeFrame { bytes }.into())
    }
}

impl Assemble for ConnIdChangeFrame {
    fn assemble(&self) -> BytesMut {
        self.bytes.clone().into()
    }
}

impl Debug for ConnIdChangeFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let old_cid = self.header().old_cid;
        let new_cid = self.header().new_cid;
        f.debug_struct("ConnIdChange")
            .field("old_cid", &old_cid)
            .field("new_cid", &new_cid)
            .finish()
    }
}

#[derive(Debug, AsBytes, FromZeroes, FromBytes)]
#[repr(C, packed)]
pub struct FlowControlHeader {
    pub type_id: u8,
    pub window_size: u32,
}

#[derive(Clone)]
pub struct FlowControlFrame {
    bytes: Bytes,
}

impl Size for FlowControlFrame {
    #[inline(always)]
    fn size(&self) -> usize {
        size_of::<FlowControlHeader>()
    }
}

impl FlowControlFrame {
    const TYPE_ID: u8 = 3;

    pub fn new(window_size: u32) -> Self {
        let header = FlowControlHeader {
            type_id: Self::TYPE_ID,
            window_size,
        };
        let bytes = BytesMut::from(AsBytes::as_bytes(&header)).into();
        FlowControlFrame { bytes }
    }

    pub fn header(&self) -> &FlowControlHeader {
        FlowControlHeader::ref_from(self.bytes.as_ref())
            .expect("Failed to reference FlowControlHeader")
    }

    pub fn type_id(&self) -> u8 {
        self.header().type_id
    }

    pub fn window_size(&self) -> u32 {
        self.header().window_size
    }
}

impl Parse for FlowControlFrame {
    fn parse(bytes: &mut Bytes) -> Result<Frame, anyhow::Error> {
        // TODO bounds check
        let bytes = bytes.split_to(size_of::<FlowControlHeader>());
        Ok(FlowControlFrame { bytes }.into())
    }
}

impl Assemble for FlowControlFrame {
    fn assemble(&self) -> BytesMut {
        self.bytes.clone().into()
    }
}

impl Debug for FlowControlFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FlowControl")
            .field("window_size", &self.window_size())
            .finish()
    }
}

#[derive(Debug, AsBytes, FromZeroes, FromBytes)]
#[repr(C, packed)]
pub struct AnswerHeader {
    pub type_id: u8,
    pub stream_id: u16,
}

#[derive(Clone)]
pub struct AnswerFrame {
    pub header_bytes: Bytes,
    pub payload_bytes: Bytes,
}

impl Size for AnswerFrame {
    #[inline(always)]
    fn size(&self) -> usize {
        size_of::<AnswerHeader>() + 2 + self.payload_bytes.len()
    }
}

impl AnswerFrame {
    const TYPE_ID: u8 = 4;

    pub fn new(stream_id: u16, payload: Bytes) -> Self {
        let header = AnswerHeader {
            type_id: Self::TYPE_ID,
            stream_id,
        };
        let header_bytes = BytesMut::from(AsBytes::as_bytes(&header)).into();
        AnswerFrame {
            header_bytes,
            payload_bytes: payload,
        }
    }

    pub fn header(&self) -> &AnswerHeader {
        AnswerHeader::ref_from(self.header_bytes.as_ref())
            .expect("Failed to reference AnswerHeader")
    }

    pub fn type_id(&self) -> u8 {
        self.header().type_id
    }

    pub fn stream_id(&self) -> u16 {
        self.header().stream_id
    }

    pub fn payload(&self) -> &Bytes {
        &self.payload_bytes
    }
}

impl Parse for AnswerFrame {
    fn parse(bytes: &mut Bytes) -> Result<Frame, anyhow::Error> {
        // TODO bounds check
        let header_bytes = bytes.split_to(size_of::<AnswerHeader>());
        let length_bytes = bytes.split_to(2);
        let payload_length = length_bytes[0] as usize | (length_bytes[1] as usize) << 8;
        let payload_bytes = bytes.split_to(payload_length);
        Ok(AnswerFrame {
            header_bytes,
            payload_bytes,
        }
        .into())
    }
}

impl Assemble for AnswerFrame {
    fn assemble(&self) -> BytesMut {
        let mut bytes = BytesMut::from(self.header_bytes.clone());
        bytes.extend_from_slice(&self.payload_bytes.len().to_le_bytes()[..2]);
        bytes.extend_from_slice(&self.payload_bytes);
        bytes
    }
}

impl Debug for AnswerFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Answer")
            .field("stream_id", &self.stream_id())
            .field("payload", &self.payload())
            .finish()
    }
}

#[derive(Debug, AsBytes, FromZeroes, FromBytes)]
#[repr(C, packed)]
pub struct ErrorHeader {
    pub type_id: u8,
    pub stream_id: u16,
}

#[derive(Clone)]
pub struct ErrorFrame {
    pub header_bytes: Bytes,
    pub payload_bytes: Bytes,
}

impl Size for ErrorFrame {
    #[inline(always)]
    fn size(&self) -> usize {
        size_of::<ErrorHeader>() + 2 + self.payload_bytes.len()
    }
}

impl ErrorFrame {
    const TYPE_ID: u8 = 5;

    pub fn new(stream_id: u16, message: &str) -> Self {
        let header = ErrorHeader {
            type_id: Self::TYPE_ID,
            stream_id,
        };
        let header_bytes = BytesMut::from(header.as_bytes()).into();
        let payload_bytes = Bytes::copy_from_slice(message.as_bytes());
        ErrorFrame {
            header_bytes,
            payload_bytes,
        }
    }

    pub fn header(&self) -> &ErrorHeader {
        ErrorHeader::ref_from(self.header_bytes.as_ref()).expect("Failed to reference ErrorHeader")
    }

    pub fn type_id(&self) -> u8 {
        self.header().type_id
    }

    pub fn stream_id(&self) -> u16 {
        self.header().stream_id
    }

    pub fn message(&self) -> &str {
        from_utf8(self.payload_bytes.as_ref()).expect("Failed to parse message")
    }
}

impl Parse for ErrorFrame {
    fn parse(bytes: &mut Bytes) -> Result<Frame, anyhow::Error> {
        // TODO bounds check
        let header_bytes = bytes.split_to(size_of::<ErrorHeader>());
        let length_bytes = bytes.split_to(2);
        let payload_length = length_bytes[0] as usize | (length_bytes[1] as usize) << 8;
        let payload_bytes = bytes.split_to(payload_length);
        Ok(ErrorFrame {
            header_bytes,
            payload_bytes,
        }
        .into())
    }
}

impl Assemble for ErrorFrame {
    fn assemble(&self) -> BytesMut {
        let mut bytes = BytesMut::from(self.header_bytes.clone());
        bytes.extend_from_slice(&self.payload_bytes.len().to_le_bytes()[..2]);
        bytes.extend_from_slice(&self.payload_bytes);
        bytes
    }
}

impl Debug for ErrorFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Error")
            .field("stream_id", &self.stream_id())
            .field("message", &self.message())
            .finish()
    }
}

fn six_u8_to_u64(array: &[u8; 6]) -> u64 {
    let mut result: [u8; 8] = [0; 8];
    result[..6].copy_from_slice(array);
    u64::from_le_bytes(result)
}

fn u64_to_six_u8(value: u64) -> [u8; 6] {
    let mut result: [u8; 6] = [0; 6];
    result.copy_from_slice(&value.as_bytes()[..6]);
    result
}

#[derive(Debug, AsBytes, FromZeroes, FromBytes)]
#[repr(C, packed)]
pub struct DataHeader {
    pub type_id: u8,
    pub stream_id: u16,
    pub offset: [u8; 6],
}

#[derive(Clone)]
pub struct DataFrame {
    pub header_bytes: Bytes,
    pub payload_bytes: Bytes,
}

impl Size for DataFrame {
    #[inline(always)]
    fn size(&self) -> usize {
        size_of::<DataHeader>() + 2 + self.payload_bytes.len()
    }
}

impl DataFrame {
    const TYPE_ID: u8 = 6;

    pub fn new(stream_id: u16, offset: u64, payload: Bytes) -> Self {
        let header = DataHeader {
            type_id: Self::TYPE_ID,
            stream_id,
            offset: u64_to_six_u8(offset),
        };
        let header_bytes = BytesMut::from(AsBytes::as_bytes(&header)).into();
        DataFrame {
            header_bytes,
            payload_bytes: payload,
        }
    }

    pub fn header(&self) -> &DataHeader {
        DataHeader::ref_from(self.header_bytes.as_ref()).expect("Failed to reference DataHeader")
    }

    pub fn type_id(&self) -> u8 {
        self.header().type_id
    }

    pub fn stream_id(&self) -> u16 {
        self.header().stream_id
    }

    pub fn offset(&self) -> u64 {
        six_u8_to_u64(&self.header().offset)
    }

    pub fn length(&self) -> u64 {
        self.payload_bytes.len() as u64
    }

    pub fn payload(&self) -> &Bytes {
        &self.payload_bytes
    }
}

impl Parse for DataFrame {
    fn parse(bytes: &mut Bytes) -> Result<Frame, anyhow::Error> {
        // TODO bounds check
        let header_bytes = bytes.split_to(size_of::<DataHeader>());
        // TODO put this into a helper function of the header struct,
        //      or define a custom u24 type
        let length_bytes = bytes.split_to(2);
        let payload_length = length_bytes[0] as usize | (length_bytes[1] as usize) << 8;
        let payload_bytes = bytes.split_to(payload_length);
        Ok(DataFrame {
            header_bytes,
            payload_bytes,
        }
        .into())
    }
}

impl Assemble for DataFrame {
    fn assemble(&self) -> BytesMut {
        let mut bytes = BytesMut::from(self.header_bytes.clone());
        bytes.extend_from_slice(&self.payload_bytes.len().to_le_bytes()[..2]);
        bytes.extend_from_slice(&self.payload_bytes);
        bytes
    }
}

impl Debug for DataFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Data")
            .field("stream_id", &self.stream_id())
            .field("offset", &self.offset())
            .field("payload", &self.payload())
            .finish()
    }
}

#[derive(Debug, AsBytes, FromZeroes, FromBytes)]
#[repr(C, packed)]
pub struct ReadHeader {
    pub type_id: u8,
    pub stream_id: u16,
    pub flags: u8,
    pub offset: [u8; 6],
    pub length: [u8; 6],
    pub checksum: u32,
}

#[derive(Clone)]
pub struct ReadFrame {
    pub header_bytes: Bytes,
    pub payload_bytes: Bytes,
}

impl Size for ReadFrame {
    #[inline(always)]
    fn size(&self) -> usize {
        size_of::<ReadHeader>() + 2 + self.payload_bytes.len()
    }
}

impl ReadFrame {
    const TYPE_ID: u8 = 7;

    pub fn new(
        stream_id: u16,
        flags: u8,
        offset: u64,
        length: u64,
        checksum: u32,
        path: &Path,
    ) -> Self {
        let header = ReadHeader {
            type_id: Self::TYPE_ID,
            stream_id,
            flags,
            offset: u64_to_six_u8(offset),
            length: u64_to_six_u8(length),
            checksum,
        };
        let header_bytes = BytesMut::from(AsBytes::as_bytes(&header)).into();
        let payload_bytes = Bytes::copy_from_slice(
            path.to_str()
                .expect("Failed to convert path to string")
                .as_bytes(),
        );
        ReadFrame {
            header_bytes,
            payload_bytes,
        }
    }

    pub fn header(&self) -> &ReadHeader {
        ReadHeader::ref_from(self.header_bytes.as_ref()).expect("Failed to reference ReadHeader")
    }

    pub fn type_id(&self) -> u8 {
        self.header().type_id
    }

    pub fn stream_id(&self) -> u16 {
        self.header().stream_id
    }

    pub fn flags(&self) -> u8 {
        self.header().flags
    }

    pub fn offset(&self) -> u64 {
        six_u8_to_u64(&self.header().offset)
    }

    pub fn length(&self) -> u64 {
        six_u8_to_u64(&self.header().length)
    }

    pub fn checksum(&self) -> u32 {
        self.header().checksum
    }

    pub fn path(&self) -> &Path {
        Path::new(from_utf8(self.payload_bytes.as_ref()).expect("Failed to parse path"))
    }
}

impl Parse for ReadFrame {
    fn parse(bytes: &mut Bytes) -> Result<Frame, anyhow::Error> {
        // TODO bounds check
        let header_bytes = bytes.split_to(size_of::<ReadHeader>());
        let length_bytes = bytes.split_to(2);
        let payload_length = length_bytes[0] as usize | (length_bytes[1] as usize) << 8;
        let payload_bytes = bytes.split_to(payload_length);
        Ok(ReadFrame {
            header_bytes,
            payload_bytes,
        }
        .into())
    }
}

impl Assemble for ReadFrame {
    fn assemble(&self) -> BytesMut {
        let mut bytes = BytesMut::from(self.header_bytes.clone());
        bytes.extend_from_slice(&self.payload_bytes.len().to_le_bytes()[..2]);
        bytes.extend_from_slice(&self.payload_bytes);
        bytes
    }
}

impl Debug for ReadFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Read")
            .field("stream_id", &self.stream_id())
            .field("flags", &self.flags())
            .field("offset", &self.offset())
            .field("length", &self.length())
            .field("checksum", &self.checksum())
            .field("path", &self.path())
            .finish()
    }
}

#[derive(Debug, AsBytes, FromZeroes, FromBytes)]
#[repr(C, packed)]
pub struct WriteHeader {
    pub type_id: u8,
    pub stream_id: u16,
    pub offset: [u8; 6],
    pub length: [u8; 6],
}

#[derive(Clone)]
pub struct WriteFrame {
    pub header_bytes: Bytes,
    pub payload_bytes: Bytes,
}

impl Size for WriteFrame {
    #[inline(always)]
    fn size(&self) -> usize {
        size_of::<WriteHeader>() + 2 + self.payload_bytes.len()
    }
}

impl WriteFrame {
    const TYPE_ID: u8 = 8;

    pub fn new(stream_id: u16, offset: u64, length: u64, path: &Path) -> Self {
        let header = WriteHeader {
            type_id: Self::TYPE_ID,
            stream_id,
            offset: u64_to_six_u8(offset),
            length: u64_to_six_u8(length),
        };
        let header_bytes = BytesMut::from(AsBytes::as_bytes(&header)).into();
        let payload_bytes = Bytes::copy_from_slice(
            path.to_str()
                .expect("Failed to convert path to string")
                .as_bytes(),
        );
        WriteFrame {
            header_bytes,
            payload_bytes,
        }
    }

    pub fn header(&self) -> &WriteHeader {
        WriteHeader::ref_from(self.header_bytes.as_ref()).expect("Failed to reference WriteHeader")
    }

    pub fn type_id(&self) -> u8 {
        self.header().type_id
    }

    pub fn stream_id(&self) -> u16 {
        self.header().stream_id
    }

    pub fn offset(&self) -> u64 {
        six_u8_to_u64(&self.header().offset)
    }

    pub fn length(&self) -> u64 {
        six_u8_to_u64(&self.header().length)
    }

    pub fn path(&self) -> &Path {
        Path::new(from_utf8(self.payload_bytes.as_ref()).expect("Failed to parse path"))
    }
}

impl Parse for WriteFrame {
    fn parse(bytes: &mut Bytes) -> Result<Frame, anyhow::Error> {
        // TODO bounds check
        let header_bytes = bytes.split_to(size_of::<WriteHeader>());
        let length_bytes = bytes.split_to(2);
        let payload_length = length_bytes[0] as usize | (length_bytes[1] as usize) << 8;
        let payload_bytes = bytes.split_to(payload_length);
        Ok(WriteFrame {
            header_bytes,
            payload_bytes,
        }
        .into())
    }
}

impl Assemble for WriteFrame {
    fn assemble(&self) -> BytesMut {
        let mut bytes = BytesMut::from(self.header_bytes.clone());
        bytes.extend_from_slice(&self.payload_bytes.len().to_le_bytes()[..2]);
        bytes.extend_from_slice(&self.payload_bytes);
        bytes
    }
}

impl Debug for WriteFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Write")
            .field("stream_id", &self.stream_id())
            .field("offset", &self.offset())
            .field("length", &self.length())
            .field("path", &self.path())
            .finish()
    }
}

#[derive(Debug, AsBytes, FromZeroes, FromBytes)]
#[repr(C, packed)]
pub struct ChecksumHeader {
    pub type_id: u8,
    pub stream_id: u16,
}

#[derive(Clone)]
pub struct ChecksumFrame {
    pub header_bytes: Bytes,
    pub payload_bytes: Bytes,
}

impl Size for ChecksumFrame {
    #[inline(always)]
    fn size(&self) -> usize {
        size_of::<ChecksumHeader>() + 2 + self.payload_bytes.len()
    }
}

impl ChecksumFrame {
    const TYPE_ID: u8 = 9;

    pub fn new(stream_id: u16, path: &Path) -> Self {
        let header = ChecksumHeader {
            type_id: Self::TYPE_ID,
            stream_id,
        };
        let header_bytes = BytesMut::from(AsBytes::as_bytes(&header)).into();
        let payload_bytes = Bytes::copy_from_slice(
            path.to_str()
                .expect("Failed to convert path to string")
                .as_bytes(),
        );
        ChecksumFrame {
            header_bytes,
            payload_bytes,
        }
    }

    pub fn header(&self) -> &ChecksumHeader {
        ChecksumHeader::ref_from(self.header_bytes.as_ref())
            .expect("Failed to reference ChecksumHeader")
    }

    pub fn type_id(&self) -> u8 {
        self.header().type_id
    }

    pub fn stream_id(&self) -> u16 {
        self.header().stream_id
    }

    pub fn path(&self) -> &Path {
        Path::new(from_utf8(self.payload_bytes.as_ref()).expect("Failed to parse path"))
    }
}

impl Parse for ChecksumFrame {
    fn parse(bytes: &mut Bytes) -> Result<Frame, anyhow::Error> {
        // TODO bounds check
        let header_bytes = bytes.split_to(size_of::<ChecksumHeader>());
        let length_bytes = bytes.split_to(2);
        let payload_length = length_bytes[0] as usize | (length_bytes[1] as usize) << 8;
        let payload_bytes = bytes.split_to(payload_length);
        Ok(ChecksumFrame {
            header_bytes,
            payload_bytes,
        }
        .into())
    }
}

impl Assemble for ChecksumFrame {
    fn assemble(&self) -> BytesMut {
        let mut bytes = BytesMut::from(self.header_bytes.clone());
        bytes.extend_from_slice(&self.payload_bytes.len().to_le_bytes()[..2]);
        bytes.extend_from_slice(&self.payload_bytes);
        bytes
    }
}

impl Debug for ChecksumFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Checksum")
            .field("stream_id", &self.stream_id())
            .field("path", &self.path())
            .finish()
    }
}

#[derive(Debug, AsBytes, FromZeroes, FromBytes)]
#[repr(C, packed)]
pub struct StatHeader {
    pub type_id: u8,
    pub stream_id: u16,
}

#[derive(Clone)]
pub struct StatFrame {
    pub header_bytes: Bytes,
    pub payload_bytes: Bytes,
}

impl Size for StatFrame {
    #[inline(always)]
    fn size(&self) -> usize {
        size_of::<StatHeader>() + 2 + self.payload_bytes.len()
    }
}

impl StatFrame {
    const TYPE_ID: u8 = 10;

    pub fn new(stream_id: u16, path: &Path) -> Self {
        let header = StatHeader {
            type_id: Self::TYPE_ID,
            stream_id,
        };
        let header_bytes = BytesMut::from(AsBytes::as_bytes(&header)).into();
        let payload_bytes = Bytes::copy_from_slice(
            path.to_str()
                .expect("Failed to convert path to string")
                .as_bytes(),
        );
        StatFrame {
            header_bytes,
            payload_bytes,
        }
    }

    pub fn header(&self) -> &StatHeader {
        StatHeader::ref_from(self.header_bytes.as_ref()).expect("Failed to reference StatHeader")
    }

    pub fn type_id(&self) -> u8 {
        self.header().type_id
    }

    pub fn stream_id(&self) -> u16 {
        self.header().stream_id
    }

    pub fn path(&self) -> &Path {
        Path::new(from_utf8(self.payload_bytes.as_ref()).expect("Failed to parse path"))
    }
}

impl Parse for StatFrame {
    fn parse(bytes: &mut Bytes) -> Result<Frame, anyhow::Error> {
        // TODO bounds check
        let header_bytes = bytes.split_to(size_of::<StatHeader>());
        let length_bytes = bytes.split_to(2);
        let payload_length = length_bytes[0] as usize | (length_bytes[1] as usize) << 8;
        let payload_bytes = bytes.split_to(payload_length);
        Ok(StatFrame {
            header_bytes,
            payload_bytes,
        }
        .into())
    }
}

impl Assemble for StatFrame {
    fn assemble(&self) -> BytesMut {
        let mut bytes = BytesMut::from(self.header_bytes.clone());
        bytes.extend_from_slice(&self.payload_bytes.len().to_le_bytes()[..2]);
        bytes.extend_from_slice(&self.payload_bytes);
        bytes
    }
}

impl Debug for StatFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Stat")
            .field("stream_id", &self.stream_id())
            .field("path", &self.path())
            .finish()
    }
}

#[derive(Debug, AsBytes, FromZeroes, FromBytes)]
#[repr(C, packed)]
pub struct ListHeader {
    pub type_id: u8,
    pub stream_id: u16,
}

#[derive(Clone)]
pub struct ListFrame {
    pub header_bytes: Bytes,
    pub payload_bytes: Bytes,
}

impl Size for ListFrame {
    #[inline(always)]
    fn size(&self) -> usize {
        size_of::<ListHeader>() + 2 + self.payload_bytes.len()
    }
}

impl ListFrame {
    const TYPE_ID: u8 = 11;

    pub fn new(stream_id: u16, path: &Path) -> Self {
        let header = ListHeader {
            type_id: Self::TYPE_ID,
            stream_id,
        };
        let header_bytes = BytesMut::from(AsBytes::as_bytes(&header)).into();
        let payload_bytes = Bytes::copy_from_slice(
            path.to_str()
                .expect("Failed to convert path to string")
                .as_bytes(),
        );
        ListFrame {
            header_bytes,
            payload_bytes,
        }
    }

    pub fn header(&self) -> &ListHeader {
        ListHeader::ref_from(self.header_bytes.as_ref()).expect("Failed to reference ListHeader")
    }

    pub fn type_id(&self) -> u8 {
        self.header().type_id
    }

    pub fn stream_id(&self) -> u16 {
        self.header().stream_id
    }

    pub fn path(&self) -> &Path {
        Path::new(from_utf8(self.payload_bytes.as_ref()).expect("Failed to parse path"))
    }
}

impl Parse for ListFrame {
    fn parse(bytes: &mut Bytes) -> Result<Frame, anyhow::Error> {
        // TODO bounds check
        let header_bytes = bytes.split_to(size_of::<ListHeader>());
        let length_bytes = bytes.split_to(2);
        let payload_length = length_bytes[0] as usize | (length_bytes[1] as usize) << 8;
        let payload_bytes = bytes.split_to(payload_length);
        Ok(ListFrame {
            header_bytes,
            payload_bytes,
        }
        .into())
    }
}

impl Assemble for ListFrame {
    fn assemble(&self) -> BytesMut {
        let mut bytes = BytesMut::from(self.header_bytes.clone());
        bytes.extend_from_slice(&self.payload_bytes.len().to_le_bytes()[..2]);
        bytes.extend_from_slice(&self.payload_bytes);
        bytes
    }
}

impl Debug for ListFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("List")
            .field("stream_id", &self.stream_id())
            .field("path", &self.path())
            .finish()
    }
}

#[derive(Clone)]
pub struct Packet {
    header_bytes: Bytes,
    pub frames: Vec<Frame>,
}

impl Size for Packet {
    #[inline(always)]
    fn size(&self) -> usize {
        size_of::<PacketHeader>() + self.frames.iter().map(|frame| frame.size()).sum::<usize>()
    }
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
    // TODO add convenience getters
    pub fn new(connection_id: u32, packet_id: u32) -> Self {
        let header = PacketHeader {
            version: VERSION,
            connection_id,
            packet_id,
            checksum: [0; 3],
        };
        let header_bytes = BytesMut::from(AsBytes::as_bytes(&header)).into();
        Packet {
            header_bytes,
            frames: Vec::new(),
        }
    }

    fn validate_checksum(bytes: &Bytes) -> bool {
        let header = PacketHeader::ref_from(&bytes[0..size_of::<PacketHeader>()])
            .expect("Failed to reference PacketHeader");
        let expected = header.checksum();
        // TODO the hasher should be cached somewhere outside of the Packet
        let mut hasher = crc32fast::Hasher::new();
        hasher.reset();
        hasher.update(&bytes[0..=8]);
        hasher.update(&[0; 3]);
        hasher.update(&bytes[12..]);
        let actual = hasher.finalize() & 0x00FFFFFF;
        expected == actual
    }

    // TODO better error handling
    pub fn parse(bytes: Bytes) -> Result<Self, anyhow::Error> {
        // TODO bounds check
        if !Self::validate_checksum(&bytes) {
            return Err(anyhow!("Checksum validation failed"));
        }
        let mut header_bytes = bytes;
        let mut frame_bytes = header_bytes.split_off(size_of::<PacketHeader>());
        let mut packet = Packet {
            header_bytes,
            frames: Vec::new(),
        };
        while !frame_bytes.is_empty() {
            let code = frame_bytes[0];
            packet.frames.push(match code {
                0 => AckFrame::parse(&mut frame_bytes)?,
                1 => ExitFrame::parse(&mut frame_bytes)?,
                2 => ConnIdChangeFrame::parse(&mut frame_bytes)?,
                3 => FlowControlFrame::parse(&mut frame_bytes)?,
                4 => AnswerFrame::parse(&mut frame_bytes)?,
                5 => ErrorFrame::parse(&mut frame_bytes)?,
                6 => DataFrame::parse(&mut frame_bytes)?,
                7 => ReadFrame::parse(&mut frame_bytes)?,
                8 => WriteFrame::parse(&mut frame_bytes)?,
                9 => ChecksumFrame::parse(&mut frame_bytes)?,
                10 => StatFrame::parse(&mut frame_bytes)?,
                11 => ListFrame::parse(&mut frame_bytes)?,
                _ => return Err(anyhow!("Unknown frame type")),
            });
        }
        Ok(packet)
    }

    pub fn parse_buf(buf: &[u8]) -> Result<Self, anyhow::Error> {
        Self::parse(Bytes::copy_from_slice(buf))
    }

    pub fn header(&self) -> &PacketHeader {
        PacketHeader::ref_from(self.header_bytes.as_ref())
            .expect("Failed to reference PacketHeader")
    }

    pub fn version(&self) -> u8 {
        self.header().version
    }

    pub fn connection_id(&self) -> u32 {
        self.header().connection_id
    }

    pub fn packet_id(&self) -> u32 {
        self.header().packet_id
    }

    pub fn checksum(&self) -> u32 {
        self.header().checksum()
    }

    pub fn add_frame(&mut self, frame: Frame) {
        self.frames.push(frame);
    }
}

impl Assemble for Packet {
    fn assemble(&self) -> BytesMut {
        let mut bytes: BytesMut = self.header_bytes.clone().into();
        for frame in &self.frames {
            bytes.extend_from_slice(&frame.assemble());
        }
        bytes[9] = 0;
        bytes[10] = 0;
        bytes[11] = 0;
        let checksum = crc32fast::hash(&bytes) & 0x00FFFFFF;
        bytes[9] = checksum as u8;
        bytes[10] = (checksum >> 8) as u8;
        bytes[11] = (checksum >> 16) as u8;
        bytes
    }
}

#[derive(Clone)]
pub enum Frame {
    Ack(AckFrame),
    Exit(ExitFrame),
    ConnIdChange(ConnIdChangeFrame),
    FlowControl(FlowControlFrame),
    Answer(AnswerFrame),
    Error(ErrorFrame),
    Data(DataFrame),
    Read(ReadFrame),
    Write(WriteFrame),
    Checksum(ChecksumFrame),
    Stat(StatFrame),
    List(ListFrame),
}

impl Frame {
    pub fn stream_id(&self) -> u16 {
        match self {
            Frame::Ack(_) => 0,
            Frame::Exit(_) => 0,
            Frame::ConnIdChange(_) => 0,
            Frame::FlowControl(_) => 0,
            Frame::Answer(frame) => frame.stream_id(),
            Frame::Error(frame) => frame.stream_id(),
            Frame::Data(frame) => frame.stream_id(),
            Frame::Read(frame) => frame.stream_id(),
            Frame::Write(frame) => frame.stream_id(),
            Frame::Checksum(frame) => frame.stream_id(),
            Frame::Stat(frame) => frame.stream_id(),
            Frame::List(frame) => frame.stream_id(),
        }
    }
}

impl Debug for Frame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Frame::Ack(frame) => frame.fmt(f),
            Frame::Exit(frame) => frame.fmt(f),
            Frame::ConnIdChange(frame) => frame.fmt(f),
            Frame::FlowControl(frame) => frame.fmt(f),
            Frame::Answer(frame) => frame.fmt(f),
            Frame::Error(frame) => frame.fmt(f),
            Frame::Data(frame) => frame.fmt(f),
            Frame::Read(frame) => frame.fmt(f),
            Frame::Write(frame) => frame.fmt(f),
            Frame::Checksum(frame) => frame.fmt(f),
            Frame::Stat(frame) => frame.fmt(f),
            Frame::List(frame) => frame.fmt(f),
        }
    }
}

impl Assemble for Frame {
    fn assemble(&self) -> BytesMut {
        match self {
            Frame::Ack(frame) => frame.assemble(),
            Frame::Exit(frame) => frame.assemble(),
            Frame::ConnIdChange(frame) => frame.assemble(),
            Frame::FlowControl(frame) => frame.assemble(),
            Frame::Answer(frame) => frame.assemble(),
            Frame::Error(frame) => frame.assemble(),
            Frame::Data(frame) => frame.assemble(),
            Frame::Read(frame) => frame.assemble(),
            Frame::Write(frame) => frame.assemble(),
            Frame::Checksum(frame) => frame.assemble(),
            Frame::Stat(frame) => frame.assemble(),
            Frame::List(frame) => frame.assemble(),
        }
    }
}

impl Size for Frame {
    fn size(&self) -> usize {
        match self {
            Frame::Ack(frame) => frame.size(),
            Frame::Exit(frame) => frame.size(),
            Frame::ConnIdChange(frame) => frame.size(),
            Frame::FlowControl(frame) => frame.size(),
            Frame::Answer(frame) => frame.size(),
            Frame::Error(frame) => frame.size(),
            Frame::Data(frame) => frame.size(),
            Frame::Read(frame) => frame.size(),
            Frame::Write(frame) => frame.size(),
            Frame::Checksum(frame) => frame.size(),
            Frame::Stat(frame) => frame.size(),
            Frame::List(frame) => frame.size(),
        }
    }
}

impl From<AckFrame> for Frame {
    fn from(frame: AckFrame) -> Self {
        Frame::Ack(frame)
    }
}

impl From<ExitFrame> for Frame {
    fn from(frame: ExitFrame) -> Self {
        Frame::Exit(frame)
    }
}

impl From<ConnIdChangeFrame> for Frame {
    fn from(frame: ConnIdChangeFrame) -> Self {
        Frame::ConnIdChange(frame)
    }
}

impl From<FlowControlFrame> for Frame {
    fn from(frame: FlowControlFrame) -> Self {
        Frame::FlowControl(frame)
    }
}

impl From<AnswerFrame> for Frame {
    fn from(frame: AnswerFrame) -> Self {
        Frame::Answer(frame)
    }
}

impl From<ErrorFrame> for Frame {
    fn from(frame: ErrorFrame) -> Self {
        Frame::Error(frame)
    }
}

impl From<DataFrame> for Frame {
    fn from(frame: DataFrame) -> Self {
        Frame::Data(frame)
    }
}

impl From<ReadFrame> for Frame {
    fn from(frame: ReadFrame) -> Self {
        Frame::Read(frame)
    }
}

impl From<WriteFrame> for Frame {
    fn from(frame: WriteFrame) -> Self {
        Frame::Write(frame)
    }
}

impl From<ChecksumFrame> for Frame {
    fn from(frame: ChecksumFrame) -> Self {
        Frame::Checksum(frame)
    }
}

impl From<StatFrame> for Frame {
    fn from(frame: StatFrame) -> Self {
        Frame::Stat(frame)
    }
}

impl From<ListFrame> for Frame {
    fn from(frame: ListFrame) -> Self {
        Frame::List(frame)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_six_u8_to_u64() {
        let array: [u8; 6] = [6, 5, 4, 3, 2, 1];
        let value = 0x010203040506;
        assert_eq!(six_u8_to_u64(&array), value);
    }

    #[test]
    fn test_u64_to_six_u8() {
        let value = 0x010203040506;
        let array: [u8; 6] = [6, 5, 4, 3, 2, 1];
        assert_eq!(u64_to_six_u8(value), array);
    }

    #[test]
    fn test_packet_header_checksum() {
        let header = PacketHeader {
            version: 1,
            connection_id: 1,
            packet_id: 2,
            checksum: [0x1, 0x2, 0x3],
        };
        assert_eq!(header.checksum(), 0x030201);
    }

    #[test]
    fn test_data_fields() {
        let frame = DataFrame::new(1, 2, Bytes::from_static(&[1, 2, 3, 4]));
        assert_eq!(frame.stream_id(), 1);
        assert_eq!(frame.offset(), 2);
        assert_eq!(frame.payload(), &Bytes::from_static(&[1, 2, 3, 4]));
    }

    #[test]
    fn test_assemble_empty_packet() {
        let packet = Packet::new(2, 4);
        assert_eq!(
            packet.assemble(),
            Bytes::from_static(&[1, 2, 0, 0, 0, 4, 0, 0, 0, 0xd2, 0x17, 0x53])
        );
    }

    #[test]
    fn test_validate_checksum() {
        let mut packet = Packet::new(420, 42);
        let frame = AckFrame::new(1);
        packet.add_frame(frame.into());

        let mut bytes = packet.assemble();
        bytes[9] = 0;
        bytes[10] = 0;
        bytes[11] = 0;
        let checksum = crc32fast::hash(&bytes) & 0x00FFFFFF;
        bytes[9] = checksum as u8;
        bytes[10] = (checksum >> 8) as u8;
        bytes[11] = (checksum >> 16) as u8;
        let b = Bytes::from(bytes);
        assert!(Packet::validate_checksum(&b));
    }

    #[test]
    fn test_packet_assemble() {
        let mut packet = Packet::new(420, 69);
        let frame = AckFrame::new(1);
        packet.add_frame(frame.into());
        let assembled = packet.assemble();
        assert_eq!(
            assembled.len(),
            size_of::<PacketHeader>() + size_of::<AckHeader>()
        );
    }

    #[test]
    fn test_assemble_and_parse_packet() {
        let mut packet1 = Packet::new(1, 2);
        // packet1.add_frame(AckFrame::new(1, 1).into());
        packet1.add_frame(AnswerFrame::new(1, vec![1, 2, 3, 4, 5, 6, 7, 8].into()).into());
        let bytes1 = packet1.assemble();
        let packet2 = Packet::parse(bytes1.clone().into()).expect("Parsing failed");
        let bytes2 = packet2.assemble();
        assert_eq!(bytes1, bytes2);
    }

    #[test]
    fn test_assemble_and_parse_simple_packet() {
        let packet1 = Packet::new(1, 2);
        let bytes1 = packet1.assemble();
        let packet2 = Packet::parse(bytes1.clone().into()).expect("Parsing failed");
        let bytes2 = packet2.assemble();
        assert_eq!(bytes1, bytes2);
    }

    //#[test] // generates test data
    #[allow(dead_code)]
    fn write_ack_packet_to_file() {
        let mut packet = Packet::new(69, 42);
        packet.add_frame(Frame::Ack(AckFrame::new(12)));
        let bytes = packet.assemble();
        std::fs::write("./tests/data/ack_packet.bin", bytes).expect("Failed to write file");
    }

    //#[test] // generated the test data
    #[allow(dead_code)]
    fn write_ack_data_packet_to_file() {
        let mut packet = Packet::new(69, 59);
        packet.add_frame(Frame::Ack(AckFrame::new(42)));
        packet.add_frame(Frame::Data(DataFrame::new(
            1,
            2,
            "Did you ever hear the Tragedy of Darth Plagueis the Wise?"
                .as_bytes()
                .into(),
        )));
        let bytes = packet.assemble();
        std::fs::write("./tests/data/ack_data_packet.bin", bytes).expect("Failed to write file");
    }
}
