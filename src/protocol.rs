use zerocopy_derive::{AsBytes, FromBytes, FromZeroes};

#[derive(Debug, AsBytes, FromZeroes, FromBytes)]
#[repr(C, packed)]
pub struct Packet {
    pub version: u8,
    pub connection_id: u32,
    pub checksum: [u8; 3],
    // pub frames: Vec<Frame>,
}
