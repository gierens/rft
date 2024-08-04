use bytes::BytesMut;
use std::fmt::Debug;
use zerocopy::{AsBytes, FromBytes};

use crate::protocol::*;

#[derive(Debug)]
pub struct PacketMut {
    header_bytes: BytesMut,
    pub frames: Vec<FrameMut>,
}

impl PacketMut {
    pub fn new(header: PacketHeader) -> Self {
        let header_bytes = BytesMut::from(AsBytes::as_bytes(&header));
        PacketMut {
            header_bytes,
            frames: Vec::new(),
        }
    }

    pub fn header(&mut self) -> &mut PacketHeader {
        PacketHeader::mut_from(self.header_bytes.as_mut())
            .expect("Failed to reference PacketHeader")
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
pub struct FrameMut {
    header_bytes: BytesMut,
    payload_bytes: Option<BytesMut>,
}
