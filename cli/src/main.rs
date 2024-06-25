//! This is the main module of the application.
//!
//! It handles parsing command line arguments, loading environment variables,
//! and starting the appropriate user interface (simple or fancy).

use clap::Parser;
use dotenvy::dotenv;

use std::net::Ipv4Addr;

mod simple_tui;

mod fancy_tui;

/// Command line arguments for the application.
#[derive(Debug, Parser, Clone)]
pub struct Cli {
    /// What name shoudl this server have?
    #[clap(long)]
    name: String,
    /// server ip address (0.0.0.0 should be any ip the server can listen on)
    #[structopt(long = "host")]
    ip_address: Ipv4Addr,

    /// server port address
    #[structopt(long)]
    port: u16,

    /// tracker ip address
    #[structopt(long = "tracker-host")]
    tracker_ip_address: Ipv4Addr,

    /// tracker port
    #[structopt(long)]
    tracker_port: u16,

    /// maximum number of incomming connections that will be advertised when Announcing to the network.
    #[structopt(default_value = "5")]
    max_connections: u64,

    /// Use simple ui mode? (/say and /quit are the only things that work)
    #[structopt(long)]
    simple: bool,
}

/// Messages that can be sent from the UI to the client.
#[derive(Debug)]
pub enum UiClientMessage {
    /// Send a chat message.
    Say(String),
    /// Send a private message to a specific user.
    Whisper(String, String),
    /// Ping a user with a specific ID.
    Ping(String, u64),
    /// Send an extension message with an ID and content.
    Extension(u64, String),
    /// Deliver raw bytes to a specific user.
    Deliver(String, Vec<u8>),
}

/// The main entry point of the application.
///
/// This function loads environment variables, parses command line arguments,
/// and starts the appropriate user interface based on the `simple` flag.
#[tokio::main]
pub async fn main() {
    // simple_ui::do_it().await

    // read in any environment variables set in a .env file
    if let Err(err) = dotenv() {
        eprintln!("Error loading .env file: {}", err);
        return;
    }

    // Run the application, printing any errors that occur
    if let Err(err) = run().await {
        eprintln!("Error: {}", err);
    }
}

/// Runs the application with the provided command line arguments.
///
/// # Errors
///
/// Returns a `Bing2BingError` if an error occurs while starting the UI.
async fn run() -> Result<(), bing2bing_core::Bing2BingError> {
    let args = Cli::parse();

    if args.simple {
        simple_tui::start(args).await?;
    } else {
        fancy_tui::start(args).await?;
        //todo!("Need to update the fancy tui!");
    }

    Ok(())
}
