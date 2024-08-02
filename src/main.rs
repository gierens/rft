use std::net::IpAddr;
use std::path::PathBuf;

use clap::Parser;
use protocol::Packet;
use zerocopy::{AsBytes, FromBytes};

mod protocol;

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

    let packet_header = protocol::PacketHeader {
        version: 1,
        connection_id: 1,
        checksum: [2; 3],
    };
    dbg!(packet_header.as_bytes());
    let packet = Packet::parse(&packet_header.as_bytes()).expect("Parsing failed");
    dbg!(packet);
    let frame = protocol::AckFrame {
        typ: 0,
        frame_id: 1,
        stream_id: 1,
    };
    dbg!(frame.as_bytes());
    let vec = [packet_header.as_bytes(), frame.as_bytes()].concat();
    let bytes = vec.as_slice();
    dbg!(bytes.as_bytes());
    let packet = Packet::parse(&bytes).expect("Parsing failed");
    dbg!(packet);
}
