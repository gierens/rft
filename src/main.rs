use std::net::IpAddr;
use std::path::PathBuf;

use clap::Parser;

mod client;
mod conn_h;
mod loss_simulation;
mod server;
mod wire;

use client::Client;
use loss_simulation::LossSimulation;
use server::Server;

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

// TODOs:
// - port main to wire::tests
// - add more tests
// - add more error handling, particularly the bounds checks
// - improve ergonomics of packet crafting and parsing
// - start on the server and client
// - reduce boilerplate in wire module with macros
// - consider splitting wire module into multiple modules

fn main() {
    let args = Cli::parse();

    let loss_sim = LossSimulation::from_options(args.p, args.q);
    if args.server {
        Server::new(args.port, loss_sim).run();
    } else {
        Client::new(loss_sim).run(args.host.unwrap(), args.port, args.files.unwrap());
    }
}
