use std::net::IpAddr;
use std::path::PathBuf;

use clap::Parser;

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
    println!("Host: {}", args.host);
}
