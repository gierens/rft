use std::net::IpAddr;
use std::path::PathBuf;

use clap::Parser;
use protocol_old::Packet;
use runtime_sized_array::Array;
use zerocopy::{AsBytes, FromBytes};

mod parser;
mod protocol;
mod protocol_old;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(
        short,
        long,
        action,
        help = "Run in server mode, conflicts with host and files arguments.",
        conflicts_with = "host",
        conflicts_with = "files"
    )]
    server: bool,

    #[arg(help = "IP address of the server", required_unless_present = "server")]
    host: Option<IpAddr>,

    #[arg(
        short = 't',
        long,
        help = "Port to connect to, or listen on in server mode.",
        default_value = "8080"
    )]
    port: u16,

    #[arg(
        short,
        help = "Markov probability that packet lost after non-lost packet."
    )]
    p: Option<f64>,

    #[arg(short, help = "Markov probability that packet lost after lost packet.")]
    q: Option<f64>,

    #[arg(
        help = "Files to download from the server",
        required_unless_present = "server"
    )]
    files: Option<Vec<PathBuf>>,
}

fn main() {
    let args = Cli::parse();

    let packet_header = protocol_old::PacketHeader {
        version: 1,
        connection_id: 1,
        checksum: [2; 3],
    };

    let frame = protocol_old::AckFrame {
        typ: 0,
        frame_id: 1,
        stream_id: 1,
    };

    let frame2 = protocol_old::AnswerFrame {
        header: &protocol_old::AnswerHeader {
            typ: 4,
            stream_id: 1,
            frame_id: 2,
            command_frame_id: 3,
            payload_length: 8,
        },
        payload: vec![1, 2, 3, 4, 5, 6, 7, 8].into(),
    };
    let frame2_vec = frame2.as_vec();
    let vec = [packet_header.as_bytes(), frame.as_bytes(), &frame2_vec].concat();
    let bytes = vec.as_slice().to_vec();
    // dbg!(bytes.as_bytes());
    // let packet = Packet::parse_full(&bytes).expect("Parsing failed");
    // dbg!(packet);
    let bytes = bytes::Bytes::from(bytes);
    let packet = parser::PacketParser::parse(bytes).expect("Parsing failed");
    dbg!(packet);
}
