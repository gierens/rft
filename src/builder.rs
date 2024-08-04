use bytes::BytesMut;
use std::fmt::Debug;
use zerocopy::{AsBytes, FromBytes};

use crate::protocol::*;

pub struct PacketMut {
    header_bytes: BytesMut,
    pub frames: Vec<FrameMut>,
}

impl Debug for PacketMut {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PacketMut")
            .field("header", &self.header())
            .field("frames", &self.frames)
            .finish()
    }
}

impl PacketMut {
    pub fn new(header: PacketHeader) -> Self {
        let header_bytes = BytesMut::from(AsBytes::as_bytes(&header));
        PacketMut {
            header_bytes,
            frames: Vec::new(),
        }
    }

    pub fn header(&self) -> &PacketHeader {
        PacketHeader::ref_from(self.header_bytes.as_ref())
            .expect("Failed to reference PacketHeader")
    }

    pub fn header_mut(&mut self) -> &mut PacketHeader {
        PacketHeader::mut_from(self.header_bytes.as_mut())
            .expect("Failed to reference PacketHeader")
    }

    pub fn length(&self) -> usize {
        let mut length = self.header_bytes.len();
        for frame in &self.frames {
            length += frame.header_bytes.len();
            if let Some(payload_bytes) = &frame.payload_bytes {
                length += payload_bytes.len();
            }
        }
        length
    }

    pub fn assemble(&self) -> BytesMut {
        let mut bytes = self.header_bytes.clone();
        for frame in &self.frames {
            bytes.extend_from_slice(&frame.header_bytes);
            if let Some(payload_bytes) = &frame.payload_bytes {
                bytes.extend_from_slice(&payload_bytes);
            }
        }
        bytes
    }
}

#[derive(Debug)]
pub enum FramesMut<'a> {
    Ack(&'a AckFrame),
    Exit(&'a ExitFrame),
    ConnIdChange(&'a ConnIdChangeFrame),
    FlowControl(&'a FlowControlFrame),
    Answer(AnswerFrameMut<'a>),
    // Error(&'a ErrorFrame<'a>),
    // Data(&'a DataFrame<'a>),
    // Read(&'a ReadCommand<'a>),
    // Write(&'a WriteCommand<'a>),
    // Checksum(&'a ChecksumCommand<'a>),
    // Stat(&'a StatCommand<'a>),
    // List(&'a ListCommand<'a>),
}

pub struct FrameMut {
    header_bytes: BytesMut,
    payload_bytes: Option<BytesMut>,
}

impl Debug for FrameMut {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.header() {
            FramesMut::Ack(frame) => frame.fmt(f),
            FramesMut::Exit(frame) => frame.fmt(f),
            FramesMut::ConnIdChange(frame) => frame.fmt(f),
            FramesMut::FlowControl(frame) => frame.fmt(f),
            FramesMut::Answer(frame) => frame.fmt(f),
        }
    }
}

impl<'a> FrameMut {
    fn code(&self) -> u8 {
        self.header_bytes[0]
    }

    pub fn header(&'a self) -> FramesMut<'a> {
        match self.code() {
            0 => FramesMut::Ack(self.into()),
            1 => FramesMut::Exit(self.into()),
            2 => FramesMut::ConnIdChange(self.into()),
            3 => FramesMut::FlowControl(self.into()),
            4 => FramesMut::Answer(self.into()),
            _ => panic!("Unknown frame type"),
        }
    }

    pub fn payload(&self) -> Option<&BytesMut> {
        self.payload_bytes.as_ref()
    }
}

impl<'a> From<&'a FrameMut> for &'a AckFrame {
    fn from(frame: &'a FrameMut) -> Self {
        AckFrame::ref_from(frame.header_bytes.as_ref()).expect("Failed to reference AckFrameMut")
    }
}

impl From<AckFrame> for FrameMut {
    fn from(frame: AckFrame) -> Self {
        let header_bytes = BytesMut::from(AsBytes::as_bytes(&frame));
        FrameMut {
            header_bytes,
            payload_bytes: None,
        }
    }
}

impl<'a> From<&'a FrameMut> for &'a ExitFrame {
    fn from(frame: &'a FrameMut) -> Self {
        ExitFrame::ref_from(frame.header_bytes.as_ref()).expect("Failed to reference ExitFrame")
    }
}

impl From<ExitFrame> for FrameMut {
    fn from(frame: ExitFrame) -> Self {
        let header_bytes = BytesMut::from(AsBytes::as_bytes(&frame));
        FrameMut {
            header_bytes,
            payload_bytes: None,
        }
    }
}

impl<'a> From<&'a FrameMut> for &'a ConnIdChangeFrame {
    fn from(frame: &'a FrameMut) -> Self {
        ConnIdChangeFrame::ref_from(frame.header_bytes.as_ref())
            .expect("Failed to reference ConnIdChangeFrame")
    }
}

impl From<ConnIdChangeFrame> for FrameMut {
    fn from(frame: ConnIdChangeFrame) -> Self {
        let header_bytes = BytesMut::from(AsBytes::as_bytes(&frame));
        FrameMut {
            header_bytes,
            payload_bytes: None,
        }
    }
}

impl<'a> From<&'a FrameMut> for &'a FlowControlFrame {
    fn from(frame: &'a FrameMut) -> Self {
        FlowControlFrame::ref_from(frame.header_bytes.as_ref())
            .expect("Failed to reference FlowControlFrame")
    }
}

impl From<FlowControlFrame> for FrameMut {
    fn from(frame: FlowControlFrame) -> Self {
        let header_bytes = BytesMut::from(AsBytes::as_bytes(&frame));
        FrameMut {
            header_bytes,
            payload_bytes: None,
        }
    }
}

#[derive(Debug)]
pub struct AnswerFrameMut<'a> {
    pub header: &'a AnswerHeader,
    pub payload: &'a BytesMut,
}

impl<'a> From<&'a FrameMut> for AnswerFrameMut<'a> {
    fn from(frame: &'a FrameMut) -> Self {
        AnswerFrameMut {
            header: AnswerHeader::ref_from(frame.header_bytes.as_ref())
                .expect("Failed to reference AnswerFrame"),
            payload: frame.payload().expect("Missing payload in AnswerFrame"),
        }
    }
}

impl From<AnswerFrameMut<'_>> for FrameMut {
    fn from(frame: AnswerFrameMut) -> Self {
        let header_bytes = BytesMut::from(AsBytes::as_bytes(frame.header));
        FrameMut {
            header_bytes,
            payload_bytes: Some(frame.payload.clone()),
        }
    }
}
