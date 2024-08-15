use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::process::exit;
use tokio::runtime;

use clap::Parser;

mod client;
mod conn_h;
mod loss_simulation;
mod server;
#[allow(dead_code)]
mod wire;

use loss_simulation::LossSimulation;
use server::Server;

#[derive(Debug, Parser)]
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
    host: Option<Ipv4Addr>,

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

    //build async runtime
    let mut runtime = runtime::Builder::new_multi_thread();
    runtime.enable_all();

    //set num_threads //TODO: take as cli arg?
    runtime.worker_threads(8);

    let runtime = match runtime.build() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to build async runtime: {}", e);
            exit(1)
        }
    };

    let loss_sim = LossSimulation::from_options(args.p, args.q);

    let result: anyhow::Result<()> = runtime.block_on(async move {
        if args.server {
            Server::new(args.port, loss_sim).run().await?;
            Ok(())
        } else {
            let config = client::ClientConfig::new(
                args.host.unwrap(),
                args.port,
                args.files.unwrap(),
                loss_sim,
            );
            let mut client = client::Client::new(config);
            match client.start() {
                Ok(_) => Ok(()),
                Err(e) => {
                    eprintln!("Failed to start client: {}", e);
                    exit(1);
                }
            }
        }
    });

    if let Err(e) = result {
        eprintln!("Error!: {}", e);
        exit(1);
    }
}
