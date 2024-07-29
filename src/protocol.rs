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
    Ack(&'a Ack),
    Exit(&'a Exit),
}

#[derive(Debug, AsBytes, FromZeroes, FromBytes)]
#[repr(C, packed)]
pub struct Ack {
    pub typ: u8,
    pub stream_id: u16,
    pub frame_id: u32,
}

#[derive(Debug, AsBytes, FromZeroes, FromBytes)]
#[repr(C, packed)]
pub struct Exit {
    pub typ: u8,
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
                    let frame_size = size_of::<Ack>();
                    if bytes.len() - index < frame_size {
                        return Err(anyhow!("Buffer to short for ack frame"));
                    }
                    Frame::Ack(
                        Ack::ref_from(&bytes[index..index + frame_size])
                            .context("Cannot transmute ack frame")?,
                    )
                }
                1 => {
                    let frame_size = size_of::<Exit>();
                    if bytes.len() - index < frame_size {
                        return Err(anyhow!("Buffer to short for exit frame"));
                    }
                    Frame::Exit(
                        Exit::ref_from(&bytes[index..index + frame_size])
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
