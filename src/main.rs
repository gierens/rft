use std::net::IpAddr;
use std::path::PathBuf;

use clap::Parser;

mod builder;
mod parser;
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
    let mut packet = builder::PacketMut::new(packet_header);
    packet.add_frame(
        protocol::AckFrame {
            typ: 0,
            frame_id: 1,
            stream_id: 1,
        }
        .into(),
    );
    packet.add_frame(
        builder::AnswerFrameMut {
            header: &protocol::AnswerHeader {
                typ: 4,
                stream_id: 1,
                frame_id: 2,
                command_frame_id: 3,
            },
            payload: &bytes::BytesMut::from(vec![1, 2, 3, 4, 5, 6, 7, 8].as_slice()),
        }
        .into(),
    );
    packet.header_mut().version = 2;
    dbg!(&packet);
    let bytes = packet.assemble();
    dbg!(&bytes);
    let packet = parser::Packet::parse(bytes.into()).expect("Parsing failed");
    dbg!(packet);
}
