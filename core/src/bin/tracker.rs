//! This module contains the implementation of the tracker server.
//!
//! The tracker server is responsible for keeping track of the peers in the network
//! and providing information about them to other peers when they register.

use bing2bing_core::Tracker;
use clap::Parser;
use dotenvy::dotenv;
use std::net::Ipv4Addr;

/// Command line arguments for the tracker application.
#[derive(Debug, Parser, Clone)]
struct Cli {
    /// IP address to bind to (0.0.0.0 should be any IP the tracker can listen on).
    #[clap(long = "host")]
    ip_address: Ipv4Addr,

    /// The port the tracker should listen on.
    #[clap(long)]
    port: u16,
}

/// The main entry point for the tracker application.
///
/// This function performs the following steps:
/// 1. Loads environment variables from a `.env` file, if present.
/// 2. Initializes a logger using `tracing_subscriber`.
/// 3. Calls the `run` function and handles any errors.
#[tokio::main]
async fn main() {
    // Read in any environment variables set in a .env file
    if let Err(err) = dotenv() {
        eprintln!("Error loading .env file: {}", err);
        return;
    }

    // Initialize a logger
    tracing_subscriber::fmt::init();

    if let Err(err) = run().await {
        eprintln!("Error: {}", err);
    }
}

/// Runs the tracker application.
///
/// This function performs the following steps:
/// 1. Parses the command line arguments using `Cli::parse()`.
/// 2. Creates a new `Tracker` instance with the provided IP address and port.
/// 3. Starts listening for incoming connections on the tracker.
///
/// # Errors
///
/// Returns a `bing2bing_core::Bing2BingError` if an error occurs while creating or running the tracker.
async fn run() -> Result<(), bing2bing_core::Bing2BingError> {
    let args = Cli::parse();

    println!("Tracker starting with args: {:?}", args);

    let ip_address = args.ip_address.to_string();
    let port = args.port.to_string();

    let tracker = Tracker::new(&ip_address, &port).await?;
    tracker.listen().await?;

    Ok(())
}