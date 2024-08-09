use anyhow::anyhow;
use bytes::{Bytes, BytesMut};
use std::fmt::Debug;
use std::mem::size_of;
use zerocopy::{AsBytes, FromBytes};
use zerocopy_derive::{AsBytes, FromBytes, FromZeroes};

#[derive(Debug, AsBytes, FromZeroes, FromBytes)]
#[repr(C, packed)]
pub struct PacketHeader {
    pub version: u8,
    pub connection_id: u32,
    pub checksum: [u8; 3],
}

impl PacketHeader {
    pub fn checksum(&self) -> u32 {
        self.checksum[0] as u32 | (self.checksum[1] as u32) << 8 | (self.checksum[2] as u32) << 16
    }
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
    pub fn new(header: PacketHeader) -> Self {
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
        let bytes = Bytes::from(bytes);
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
                bytes.extend_from_slice(&payload_bytes);
            }
        }
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

mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn assemble_empty_packet() {
        let packet_header = PacketHeader {
            version: 1,
            connection_id: 2,
            checksum: [3, 4, 5],
        };
        let packet = Packet::new(packet_header);
        assert_eq!(
            packet.assemble(),
            Bytes::from_static(&[1, 2, 0, 0, 0, 3, 4, 5])
        );
    }
}
