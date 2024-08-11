use anyhow::anyhow;
use bytes::{Bytes, BytesMut};
use std::{fmt::Debug, path::Path};
use std::mem::size_of;
use zerocopy::{AsBytes, FromBytes, FromZeroes};
use std::str::from_utf8;

const VERSION: u8 = 1;

#[derive(Debug, AsBytes, FromZeroes, FromBytes)]
#[repr(C, packed)]
pub struct PacketHeader {
    pub version: u8,
    pub connection_id: u32,
    pub checksum: [u8; 3],
}

impl PacketHeader {
    #[allow(dead_code)]
    pub fn checksum(&self) -> u32 {
        self.checksum[0] as u32 | (self.checksum[1] as u32) << 8 | (self.checksum[2] as u32) << 16
    }
}

#[derive(Debug, AsBytes, FromZeroes, FromBytes)]
#[repr(C, packed)]
pub struct AckHeader {
    pub type_id: u8,
    pub stream_id: u16,
    pub frame_id: u32,
}

pub struct AckFrame {
    bytes: Bytes,
}

impl AckFrame {
    const TYPE_ID: u8 = 0;

    pub fn new(stream_id: u16, frame_id: u32) -> Self {
        let header = AckHeader {
            type_id: Self::TYPE_ID,
            stream_id,
            frame_id,
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

    pub fn stream_id(&self) -> u16 {
        self.header().stream_id
    }

    pub fn frame_id(&self) -> u32 {
        self.header().frame_id
    }
}

impl Debug for AckFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Ack")
            .field("stream_id", &self.stream_id())
            .field("frame_id", &self.frame_id())
            .finish()
    }
}

#[derive(Debug, AsBytes, FromZeroes, FromBytes)]
#[repr(C, packed)]
pub struct ExitHeader {
    pub type_id: u8,
}

pub struct ExitFrame {
    bytes: Bytes,
}

impl ExitFrame {
    const TYPE_ID: u8 = 1;

    pub fn new() -> Self {
        let header = ExitHeader { type_id: Self::TYPE_ID };
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

pub struct ConnIdChangeFrame {
    bytes: Bytes,
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

pub struct FlowControlFrame {
    bytes: Bytes,
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
    pub frame_id: u32,
    pub command_frame_id: u32,
}

pub struct AnswerFrame {
    pub header_bytes: Bytes,
    pub payload_bytes: Bytes,
}

impl AnswerFrame {
    const TYPE_ID: u8 = 4;

    pub fn new(stream_id: u16, frame_id: u32, command_frame_id: u32, payload: Bytes) -> Self {
        let header = AnswerHeader {
            type_id: Self::TYPE_ID,
            stream_id,
            frame_id,
            command_frame_id,
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

    pub fn typ(&self) -> u8 {
        self.header().type_id
    }

    pub fn stream_id(&self) -> u16 {
        self.header().stream_id
    }

    pub fn frame_id(&self) -> u32 {
        self.header().frame_id
    }

    pub fn command_frame_id(&self) -> u32 {
        self.header().command_frame_id
    }

    pub fn payload(&self) -> &Bytes {
        &self.payload_bytes
    }
}

impl Debug for AnswerFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Answer")
            .field("stream_id", &self.stream_id())
            .field("frame_id", &self.frame_id())
            .field("command_frame_id", &self.command_frame_id())
            .field("payload", &self.payload())
            .finish()
    }
}

#[derive(Debug, AsBytes, FromZeroes, FromBytes)]
#[repr(C, packed)]
pub struct ErrorHeader {
    pub typ: u8,
    pub stream_id: u16,
    pub frame_id: u32,
    pub command_frame_id: u32,
}

fn six_u8_to_u64(array: &[u8; 6]) -> u64 {
    let mut result: [u8; 8] = [0; 8];
    result[2..].copy_from_slice(array);
    u64::from_be_bytes(result)
}

#[derive(Debug, AsBytes, FromZeroes, FromBytes)]
#[repr(C, packed)]
pub struct DataHeader {
    pub typ: u8,
    pub stream_id: u16,
    pub frame_id: u32,
    pub offset: [u8; 6],
    pub length: [u8; 6],
}

impl DataHeader {
    pub fn offset(&self) -> u64 {
        six_u8_to_u64(&self.offset)
    }

    pub fn length(&self) -> u64 {
        six_u8_to_u64(&self.length)
    }
}

#[derive(Debug, AsBytes, FromZeroes, FromBytes)]
#[repr(C, packed)]
pub struct ReadHeader {
    pub typ: u8,
    pub stream_id: u16,
    pub frame_id: u32,
    pub flags: u8,
    pub offset: [u8; 6],
    pub length: [u8; 6],
    pub checksum: u32,
}

impl ReadHeader {
    pub fn offset(&self) -> u64 {
        six_u8_to_u64(&self.offset)
    }

    pub fn length(&self) -> u64 {
        six_u8_to_u64(&self.length)
    }
}

#[derive(Debug, AsBytes, FromZeroes, FromBytes)]
#[repr(C, packed)]
pub struct WriteHeader {
    pub typ: u8,
    pub stream_id: u16,
    pub frame_id: u32,
    pub offset: [u8; 6],
    pub length: [u8; 6],
}

impl WriteHeader {
    pub fn offset(&self) -> u64 {
        six_u8_to_u64(&self.offset)
    }

    pub fn length(&self) -> u64 {
        six_u8_to_u64(&self.length)
    }
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
pub struct StatHeader {
    pub typ: u8,
    pub stream_id: u16,
    pub frame_id: u32,
}

#[derive(Debug, AsBytes, FromZeroes, FromBytes)]
#[repr(C, packed)]
pub struct ListHeader {
    pub typ: u8,
    pub stream_id: u16,
    pub frame_id: u32,
}

#[allow(dead_code)]
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

#[allow(dead_code)]
impl Packet {
    pub fn new(header: PacketHeader) -> Self {
        let header_bytes = BytesMut::from(AsBytes::as_bytes(&header)).into();
        Packet {
            header_bytes,
            frames: Vec::new(),
        }
    }

    pub fn create(connection_id: u32) -> Self {
        let header = PacketHeader {
            version: VERSION,
            connection_id,
            checksum: [0; 3],
        };
        Self::new(header)
    }

    fn validate_checksum(bytes: &Bytes) -> bool {
        let header = PacketHeader::ref_from(&bytes[0..size_of::<PacketHeader>()])
            .expect("Failed to reference PacketHeader");
        let expected = header.checksum();
        // TODO the hasher should be cached somewhere outside of the Packet
        let mut hasher = crc32fast::Hasher::new();
        hasher.reset();
        hasher.update(&bytes[0..=4]);
        hasher.update(&[0; 3]);
        hasher.update(&bytes[8..]);
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

    pub fn add_frame(&mut self, frame: Frame) {
        self.frames.push(frame);
    }

    pub fn assemble(&self) -> BytesMut {
        let mut bytes: BytesMut = self.header_bytes.clone().into();
        for frame in &self.frames {
            bytes.extend_from_slice(&frame.header_bytes);
            if let Some(payload_bytes) = &frame.payload_bytes {
                let payload_length = payload_bytes.len() as u16;
                bytes.extend_from_slice(&payload_length.to_le_bytes());
                bytes.extend_from_slice(payload_bytes);
            }
        }
        bytes[5] = 0;
        bytes[6] = 0;
        bytes[7] = 0;
        let checksum = crc32fast::hash(&bytes) & 0x00FFFFFF;
        bytes[5] = checksum as u8;
        bytes[6] = (checksum >> 8) as u8;
        bytes[7] = (checksum >> 16) as u8;
        bytes
    }
}

#[derive(Debug)]
pub enum Frames<'a> {
    Ack(&'a AckFrame),
    Exit(&'a ExitFrame),
    ConnIdChange(&'a ConnIdChangeFrame),
    FlowControl(&'a FlowControlFrame),
    Answer(AnswerFrame<'a>),
    Error(ErrorFrame<'a>),
    Data(DataFrame<'a>),
    Read(ReadFrame<'a>),
    Write(WriteFrame<'a>),
    Checksum(ChecksumFrame<'a>),
    Stat(StatFrame<'a>),
    List(ListFrame<'a>),
}

pub struct Frame {
    header_bytes: Bytes,
    payload_bytes: Option<Bytes>,
}

impl Debug for Frame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.header() {
            Frames::Ack(frame) => frame.fmt(f),
            Frames::Exit(frame) => frame.fmt(f),
            Frames::ConnIdChange(frame) => frame.fmt(f),
            Frames::FlowControl(frame) => frame.fmt(f),
            Frames::Answer(frame) => frame.fmt(f),
            Frames::Error(frame) => frame.fmt(f),
            Frames::Data(frame) => frame.fmt(f),
            Frames::Read(frame) => frame.fmt(f),
            Frames::Write(frame) => frame.fmt(f),
            Frames::Checksum(frame) => frame.fmt(f),
            Frames::Stat(frame) => frame.fmt(f),
            Frames::List(frame) => frame.fmt(f),
        }
    }
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
            5 => Frames::Error(self.into()),
            6 => Frames::Data(self.into()),
            7 => Frames::Read(self.into()),
            8 => Frames::Write(self.into()),
            9 => Frames::Checksum(self.into()),
            10 => Frames::Stat(self.into()),
            11 => Frames::List(self.into()),
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

impl From<AckFrame> for Frame {
    fn from(frame: AckFrame) -> Self {
        let header_bytes = BytesMut::from(AsBytes::as_bytes(&frame)).into();
        Frame {
            header_bytes,
            payload_bytes: None,
        }
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

impl From<ExitFrame> for Frame {
    fn from(frame: ExitFrame) -> Self {
        let header_bytes = BytesMut::from(AsBytes::as_bytes(&frame)).into();
        Frame {
            header_bytes,
            payload_bytes: None,
        }
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

impl From<ConnIdChangeFrame> for Frame {
    fn from(frame: ConnIdChangeFrame) -> Self {
        let header_bytes = BytesMut::from(AsBytes::as_bytes(&frame)).into();
        Frame {
            header_bytes,
            payload_bytes: None,
        }
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

impl From<FlowControlFrame> for Frame {
    fn from(frame: FlowControlFrame) -> Self {
        let header_bytes = BytesMut::from(AsBytes::as_bytes(&frame)).into();
        Frame {
            header_bytes,
            payload_bytes: None,
        }
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

impl From<AnswerFrame<'_>> for Frame {
    fn from(frame: AnswerFrame) -> Self {
        let header_bytes = BytesMut::from(AsBytes::as_bytes(frame.header)).into();
        Frame {
            header_bytes,
            payload_bytes: Some(frame.payload.clone()),
        }
    }
}

#[derive(Debug)]
pub struct AnswerFrameNew<'a> {
    pub header: &'a AnswerHeader,
    pub payload: Bytes,
}

impl From<AnswerFrameNew<'_>> for Frame {
    fn from(frame: AnswerFrameNew) -> Self {
        let header_bytes = BytesMut::from(AsBytes::as_bytes(frame.header)).into();
        Frame {
            header_bytes,
            payload_bytes: Some(frame.payload),
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

#[derive(Debug)]
pub struct ErrorFrame<'a> {
    pub header: &'a ErrorHeader,
    pub payload: &'a Bytes,
}

impl ErrorFrame<'_> {
    pub fn message(&self) -> &str {
        std::str::from_utf8(self.payload.as_ref()).expect("Failed to parse message")
    }
}

impl<'a> From<&'a Frame> for ErrorFrame<'a> {
    fn from(frame: &'a Frame) -> Self {
        ErrorFrame {
            header: ErrorHeader::ref_from(frame.header_bytes.as_ref())
                .expect("Failed to reference ErrorFrame"),
            payload: frame.payload().expect("Missing payload in ErrorFrame"),
        }
    }
}

impl From<ErrorFrame<'_>> for Frame {
    fn from(frame: ErrorFrame) -> Self {
        let header_bytes = BytesMut::from(AsBytes::as_bytes(frame.header)).into();
        Frame {
            header_bytes,
            payload_bytes: Some(frame.payload.clone()),
        }
    }
}

#[derive(Debug)]
pub struct ErrorFrameNew<'a> {
    pub header: &'a ErrorHeader,
    pub payload: Bytes,
}

impl From<ErrorFrameNew<'_>> for Frame {
    fn from(frame: ErrorFrameNew) -> Self {
        let header_bytes = BytesMut::from(AsBytes::as_bytes(frame.header)).into();
        Frame {
            header_bytes,
            payload_bytes: Some(frame.payload),
        }
    }
}

impl<'a> Parse for ErrorFrame<'a> {
    fn parse(bytes: &mut Bytes) -> Result<Frame, anyhow::Error> {
        // TODO bounds check
        let header_bytes = bytes.split_to(size_of::<ErrorHeader>());
        let length_bytes = bytes.split_to(2);
        let payload_length = length_bytes[0] as usize | (length_bytes[1] as usize) << 8;
        let payload_bytes = bytes.split_to(payload_length);
        Ok(Frame {
            header_bytes,
            payload_bytes: Some(payload_bytes),
        })
    }
}

#[derive(Debug)]
pub struct DataFrame<'a> {
    pub header: &'a DataHeader,
    pub payload: &'a Bytes,
}

impl<'a> From<&'a Frame> for DataFrame<'a> {
    fn from(frame: &'a Frame) -> Self {
        DataFrame {
            header: DataHeader::ref_from(frame.header_bytes.as_ref())
                .expect("Failed to reference DataFrame"),
            payload: frame.payload().expect("Missing payload in DataFrame"),
        }
    }
}

impl From<DataFrame<'_>> for Frame {
    fn from(frame: DataFrame) -> Self {
        let header_bytes = BytesMut::from(AsBytes::as_bytes(frame.header)).into();
        Frame {
            header_bytes,
            payload_bytes: Some(frame.payload.clone()),
        }
    }
}

#[derive(Debug)]
pub struct DataFrameNew<'a> {
    pub header: &'a DataHeader,
    pub payload: Bytes,
}

impl From<DataFrameNew<'_>> for Frame {
    fn from(frame: DataFrameNew) -> Self {
        let header_bytes = BytesMut::from(AsBytes::as_bytes(frame.header)).into();
        Frame {
            header_bytes,
            payload_bytes: Some(frame.payload),
        }
    }
}

impl<'a> Parse for DataFrame<'a> {
    fn parse(bytes: &mut Bytes) -> Result<Frame, anyhow::Error> {
        // TODO bounds check
        let header_bytes = bytes.split_to(size_of::<DataHeader>());
        let header =
            DataHeader::ref_from(header_bytes.as_ref()).expect("Failed to reference DataHeader");
        // TODO put this into a helper function of the header struct,
        //      or define a custom u24 type
        let payload_length = header.length[0] as usize
            | (header.length[1] as usize) << 8
            | (header.length[2] as usize) << 16;
        let payload_bytes = bytes.split_to(payload_length);
        Ok(Frame {
            header_bytes,
            payload_bytes: Some(payload_bytes),
        })
    }
}

#[derive(Debug)]
pub struct ReadFrame<'a> {
    pub header: &'a ReadHeader,
    pub payload: &'a Bytes,
}

impl ReadFrame<'_> {
    pub fn path(&self) -> &str {
        std::str::from_utf8(self.payload.as_ref()).expect("Failed to parse path")
    }
}

impl<'a> From<&'a Frame> for ReadFrame<'a> {
    fn from(frame: &'a Frame) -> Self {
        ReadFrame {
            header: ReadHeader::ref_from(frame.header_bytes.as_ref())
                .expect("Failed to reference ReadFrame"),
            payload: frame.payload().expect("Missing payload in ReadFrame"),
        }
    }
}

impl From<ReadFrame<'_>> for Frame {
    fn from(frame: ReadFrame) -> Self {
        let header_bytes = BytesMut::from(AsBytes::as_bytes(frame.header)).into();
        Frame {
            header_bytes,
            payload_bytes: Some(frame.payload.clone()),
        }
    }
}

#[derive(Debug)]
pub struct ReadFrameNew<'a> {
    pub header: &'a ReadHeader,
    pub payload: Bytes,
}

impl From<ReadFrameNew<'_>> for Frame {
    fn from(frame: ReadFrameNew) -> Self {
        let header_bytes = BytesMut::from(AsBytes::as_bytes(frame.header)).into();
        Frame {
            header_bytes,
            payload_bytes: Some(frame.payload),
        }
    }
}

impl<'a> Parse for ReadFrame<'a> {
    fn parse(bytes: &mut Bytes) -> Result<Frame, anyhow::Error> {
        // TODO bounds check
        let header_bytes = bytes.split_to(size_of::<ReadHeader>());
        let length_bytes = bytes.split_to(2);
        let payload_length = length_bytes[0] as usize | (length_bytes[1] as usize) << 8;
        let payload_bytes = bytes.split_to(payload_length);
        Ok(Frame {
            header_bytes,
            payload_bytes: Some(payload_bytes),
        })
    }
}

#[derive(Debug)]
pub struct WriteFrame<'a> {
    pub header: &'a WriteHeader,
    pub payload: &'a Bytes,
}

impl WriteFrame<'_> {
    pub fn path(&self) -> &str {
        std::str::from_utf8(self.payload.as_ref()).expect("Failed to parse path")
    }
}

impl<'a> From<&'a Frame> for WriteFrame<'a> {
    fn from(frame: &'a Frame) -> Self {
        WriteFrame {
            header: WriteHeader::ref_from(frame.header_bytes.as_ref())
                .expect("Failed to reference WriteFrame"),
            payload: frame.payload().expect("Missing payload in WriteFrame"),
        }
    }
}

impl From<WriteFrame<'_>> for Frame {
    fn from(frame: WriteFrame) -> Self {
        let header_bytes = BytesMut::from(AsBytes::as_bytes(frame.header)).into();
        Frame {
            header_bytes,
            payload_bytes: Some(frame.payload.clone()),
        }
    }
}

#[derive(Debug)]
pub struct WriteFrameNew<'a> {
    pub header: &'a WriteHeader,
    pub payload: Bytes,
}

impl From<WriteFrameNew<'_>> for Frame {
    fn from(frame: WriteFrameNew) -> Self {
        let header_bytes = BytesMut::from(AsBytes::as_bytes(frame.header)).into();
        Frame {
            header_bytes,
            payload_bytes: Some(frame.payload),
        }
    }
}

impl<'a> Parse for WriteFrame<'a> {
    fn parse(bytes: &mut Bytes) -> Result<Frame, anyhow::Error> {
        // TODO bounds check
        let header_bytes = bytes.split_to(size_of::<WriteHeader>());
        let length_bytes = bytes.split_to(2);
        let payload_length = length_bytes[0] as usize | (length_bytes[1] as usize) << 8;
        let payload_bytes = bytes.split_to(payload_length);
        Ok(Frame {
            header_bytes,
            payload_bytes: Some(payload_bytes),
        })
    }
}

#[derive(Debug)]
pub struct ChecksumFrame<'a> {
    pub header: &'a ChecksumHeader,
    pub payload: &'a Bytes,
}

impl ChecksumFrame<'_> {
    pub fn path(&self) -> &str {
        std::str::from_utf8(self.payload.as_ref()).expect("Failed to parse path")
    }
}

impl<'a> From<&'a Frame> for ChecksumFrame<'a> {
    fn from(frame: &'a Frame) -> Self {
        ChecksumFrame {
            header: ChecksumHeader::ref_from(frame.header_bytes.as_ref())
                .expect("Failed to reference ChecksumFrame"),
            payload: frame.payload().expect("Missing payload in ChecksumFrame"),
        }
    }
}

impl From<ChecksumFrame<'_>> for Frame {
    fn from(frame: ChecksumFrame) -> Self {
        let header_bytes = BytesMut::from(AsBytes::as_bytes(frame.header)).into();
        Frame {
            header_bytes,
            payload_bytes: Some(frame.payload.clone()),
        }
    }
}

#[derive(Debug)]
pub struct ChecksumFrameNew<'a> {
    pub header: &'a ChecksumHeader,
    pub payload: Bytes,
}

impl From<ChecksumFrameNew<'_>> for Frame {
    fn from(frame: ChecksumFrameNew) -> Self {
        let header_bytes = BytesMut::from(AsBytes::as_bytes(frame.header)).into();
        Frame {
            header_bytes,
            payload_bytes: Some(frame.payload),
        }
    }
}

impl<'a> Parse for ChecksumFrame<'a> {
    fn parse(bytes: &mut Bytes) -> Result<Frame, anyhow::Error> {
        // TODO bounds check
        let header_bytes = bytes.split_to(size_of::<ChecksumHeader>());
        let length_bytes = bytes.split_to(2);
        let payload_length = length_bytes[0] as usize | (length_bytes[1] as usize) << 8;
        let payload_bytes = bytes.split_to(payload_length);
        Ok(Frame {
            header_bytes,
            payload_bytes: Some(payload_bytes),
        })
    }
}

#[derive(Debug)]
pub struct StatFrame<'a> {
    pub header: &'a StatHeader,
    pub payload: &'a Bytes,
}

impl StatFrame<'_> {
    pub fn path(&self) -> &str {
        std::str::from_utf8(self.payload.as_ref()).expect("Failed to parse path")
    }
}

impl<'a> From<&'a Frame> for StatFrame<'a> {
    fn from(frame: &'a Frame) -> Self {
        StatFrame {
            header: StatHeader::ref_from(frame.header_bytes.as_ref())
                .expect("Failed to reference StatFrame"),
            payload: frame.payload().expect("Missing payload in StatFrame"),
        }
    }
}

impl From<StatFrame<'_>> for Frame {
    fn from(frame: StatFrame) -> Self {
        let header_bytes = BytesMut::from(AsBytes::as_bytes(frame.header)).into();
        Frame {
            header_bytes,
            payload_bytes: Some(frame.payload.clone()),
        }
    }
}

#[derive(Debug)]
pub struct StatFrameNew<'a> {
    pub header: &'a StatHeader,
    pub payload: Bytes,
}

impl From<StatFrameNew<'_>> for Frame {
    fn from(frame: StatFrameNew) -> Self {
        let header_bytes = BytesMut::from(AsBytes::as_bytes(frame.header)).into();
        Frame {
            header_bytes,
            payload_bytes: Some(frame.payload),
        }
    }
}

impl<'a> Parse for StatFrame<'a> {
    fn parse(bytes: &mut Bytes) -> Result<Frame, anyhow::Error> {
        // TODO bounds check
        let header_bytes = bytes.split_to(size_of::<StatHeader>());
        let length_bytes = bytes.split_to(2);
        let payload_length = length_bytes[0] as usize | (length_bytes[1] as usize) << 8;
        let payload_bytes = bytes.split_to(payload_length);
        Ok(Frame {
            header_bytes,
            payload_bytes: Some(payload_bytes),
        })
    }
}

#[derive(Debug)]
pub struct ListFrame<'a> {
    pub header: &'a ListHeader,
    pub payload: &'a Bytes,
}

impl ListFrame<'_> {
    pub fn path(&self) -> &str {
        std::str::from_utf8(self.payload.as_ref()).expect("Failed to parse path")
    }
}

impl<'a> From<&'a Frame> for ListFrame<'a> {
    fn from(frame: &'a Frame) -> Self {
        ListFrame {
            header: ListHeader::ref_from(frame.header_bytes.as_ref())
                .expect("Failed to reference ListFrame"),
            payload: frame.payload().expect("Missing payload in ListFrame"),
        }
    }
}

impl From<ListFrame<'_>> for Frame {
    fn from(frame: ListFrame) -> Self {
        let header_bytes = BytesMut::from(AsBytes::as_bytes(frame.header)).into();
        Frame {
            header_bytes,
            payload_bytes: Some(frame.payload.clone()),
        }
    }
}

#[derive(Debug)]
pub struct ListFrameNew<'a> {
    pub header: &'a ListHeader,
    pub payload: Bytes,
}

impl From<ListFrameNew<'_>> for Frame {
    fn from(frame: ListFrameNew) -> Self {
        let header_bytes = BytesMut::from(AsBytes::as_bytes(frame.header)).into();
        Frame {
            header_bytes,
            payload_bytes: Some(frame.payload),
        }
    }
}

impl<'a> Parse for ListFrame<'a> {
    fn parse(bytes: &mut Bytes) -> Result<Frame, anyhow::Error> {
        // TODO bounds check
        let header_bytes = bytes.split_to(size_of::<ListHeader>());
        let length_bytes = bytes.split_to(2);
        let payload_length = length_bytes[0] as usize | (length_bytes[1] as usize) << 8;
        let payload_bytes = bytes.split_to(payload_length);
        Ok(Frame {
            header_bytes,
            payload_bytes: Some(payload_bytes),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_six_u8_to_u64() {
        let array: [u8; 6] = [1, 2, 3, 4, 5, 6];
        assert_eq!(six_u8_to_u64(&array), 0x010203040506);
    }

    #[test]
    fn test_packet_header_checksum() {
        let header = PacketHeader {
            version: 1,
            connection_id: 1,
            checksum: [0x1, 0x2, 0x3],
        };
        assert_eq!(header.checksum(), 0x030201);
    }

    #[test]
    fn test_data_header_offset_and_length() {
        let header = DataHeader {
            typ: 0,
            stream_id: 1,
            frame_id: 2,
            offset: [1, 2, 3, 4, 5, 6],
            length: [6, 5, 4, 3, 2, 1],
        };
        assert_eq!(header.offset(), 0x0000010203040506);
        assert_eq!(header.length(), 0x0000060504030201);
    }

    #[test]
    fn test_assemble_empty_packet() {
        let packet_header = PacketHeader {
            version: 1,
            connection_id: 2,
            checksum: [3, 4, 5],
        };
        let packet = Packet::new(packet_header);
        assert_eq!(
            packet.assemble(),
            Bytes::from_static(&[1, 2, 0, 0, 0, 0xde, 0xce, 0x17])
        );
    }

    #[test]
    fn test_validate_checksum() {
        let header = PacketHeader {
            version: 1,
            connection_id: 420,
            checksum: [0, 0, 0],
        };
        let mut packet = Packet::new(header);
        let frame = Frame::from(AckFrame {
            typ: 0,
            stream_id: 1,
            frame_id: 1,
        });
        packet.add_frame(frame);

        let mut bytes = packet.assemble();
        bytes[5] = 0;
        bytes[6] = 0;
        bytes[7] = 0;
        let checksum = crc32fast::hash(&bytes) & 0x00FFFFFF;
        bytes[5] = checksum as u8;
        bytes[6] = (checksum >> 8) as u8;
        bytes[7] = (checksum >> 16) as u8;
        let b = Bytes::from(bytes);
        assert!(Packet::validate_checksum(&b));
    }

    #[test]
    fn test_packet_assemble() {
        let header = PacketHeader {
            version: 1,
            connection_id: 420,
            checksum: [0, 0, 0],
        };
        let mut packet = Packet::new(header);
        let frame = Frame::from(AckFrame {
            typ: 0,
            stream_id: 1,
            frame_id: 1,
        });
        packet.add_frame(frame);
        let assembled = packet.assemble();
        assert_eq!(
            assembled.len(),
            size_of::<PacketHeader>() + size_of::<AckFrame>()
        );
    }

    #[test]
    fn test_assemble_and_parse_packet() {
        let packet_header = PacketHeader {
            version: 1,
            connection_id: 1,
            checksum: [0x3a, 0x9c, 0x4b],
        };
        let mut packet1 = Packet::new(packet_header);
        packet1.add_frame(
            AckFrame {
                typ: 0,
                frame_id: 1,
                stream_id: 1,
            }
            .into(),
        );
        packet1.add_frame(
            AnswerFrameNew {
                header: &AnswerHeader {
                    typ: 4,
                    stream_id: 1,
                    frame_id: 2,
                    command_frame_id: 3,
                },
                payload: bytes::Bytes::from(vec![1, 2, 3, 4, 5, 6, 7, 8]),
            }
            .into(),
        );
        let bytes1 = packet1.assemble();
        let packet2 = Packet::parse(bytes1.clone().into()).expect("Parsing failed");
        let bytes2 = packet2.assemble();
        assert_eq!(bytes1, bytes2);
    }

    #[test]
    fn test_assemble_and_parse_simple_packet() {
        let packet_header = PacketHeader {
            version: 1,
            connection_id: 1,
            checksum: [0x3a, 0x9c, 0x4b],
        };
        let packet1 = Packet::new(packet_header);
        let bytes1 = packet1.assemble();
        let packet2 = Packet::parse(bytes1.clone().into()).expect("Parsing failed");
        let bytes2 = packet2.assemble();
        assert_eq!(bytes1, bytes2);
    }
}
