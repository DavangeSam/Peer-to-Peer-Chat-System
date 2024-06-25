//! A simple, but relatively extensible peer-to-peer chat system.
//! Makes heavy use of tokio for async programming.
//! Has the base functionality to enable shortest path algorithms (e.g., Dijkstra's).
//!
//! The basic is that a given peer is composed of two parts:
//!
//! 1. A [Client]
//! 2. A [Server]
//!
//!
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_serde::formats::SymmetricalJson;
use tracing::instrument;

// Exports
mod peer;
use peer::Peer;

mod connection;
pub use connection::Connection;

mod peer_map;

mod server;
pub use server::Server;

mod client;
pub use client::Client;

pub mod tracker;
pub use tracker::Tracker;

mod parser;
use parser::Parser;

pub mod cmd;
pub use cmd::Bing2BingCommand;

mod util;

pub type Bing2BingError = Box<dyn std::error::Error + Send + Sync>;
pub type ClientTxChannel = async_channel::Sender<ClientServerMessage>;
pub type ClientRxChannel = async_channel::Receiver<ClientServerMessage>;
pub type ServerTxChannel = async_channel::Sender<ClientServerMessage>;
pub type ServerRxChannel = async_channel::Receiver<ClientServerMessage>;
type PeerTxChannel = mpsc::UnboundedSender<PeerControlMessage>;
type PeerRxChannel = mpsc::UnboundedReceiver<PeerControlMessage>;

#[derive(Debug)]
pub enum ClientServerMessage {
    /// A [`Say`] event with a sender and the message that was said.
    Say((String, String)),
    /// A [`Whisper`] event with a sender, a destination, and the message that was whispered.
    Whisper((String, String, String)),

    /// A ['Ping`] event with a sender, a destination, and a sent at timestamp
    Ping((String, String, u64)),

    /// A [`Pong`] event with a sender the original sent at timestamp that was sent along with the [`Ping`].
    Pong((String, u64)),

    /// A [`Extension`] event with a sender, an extension id, and a payload.
    Extension((String, u64, Bing2BingFrame)),

    /// A [`Deliver`] event with a sender, a destination, and the data to deliver.
    Deliver((String, String, Vec<u8>)),

    /// A [`Broadcast`] event with a source, and the data to broadcast.
    Broadcast((String, Bytes))
}

#[derive(Debug)]
pub(crate) enum PeerControlMessage {
    ShutDown,
    Frame(Bing2BingFrame),
}

pub(crate) type Framed = tokio_serde::SymmetricallyFramed<
    BingLengthDelimitedCodec,
    Bing2BingFrame,
    SymmetricalJson<Bing2BingFrame>,
>;

type BingLengthDelimitedCodec =
    tokio_util::codec::Framed<TcpStream, tokio_util::codec::LengthDelimitedCodec>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Bing2BingFrame {
    /// A string
    Text(String),
    /// An error type
    Error(String),
    /// A numeric type
    Number(u64),
    /// Raw bytes
    Bulk(Vec<u8>),
    /// true/false
    Bool(bool),
    /// Null value
    Null,
    /// An array of frames
    Array(Vec<Bing2BingFrame>),
    /// A 64 bit float
    Float(f64),
}

/// Initializes a new "peer." I.e., creates a [`Client`] and a [`Server`] pair.
/// Upon return, the [`Server`] will be ready to be [Server::start()]ed.
#[instrument(level = "trace")]
pub async fn init(name: &str, ip_address: &str, port: u16) -> (Client, Server) {
    let (server_tx, server_rx) = async_channel::unbounded();
    // let client = Client::new()
    let (client_tx, rx2) = async_channel::unbounded();
    let client = Client::new(name.to_string(), server_tx.clone(), rx2);
    let server = Server::new(name, ip_address, &port.to_string(), client_tx, server_rx)
        .await
        .unwrap();

    (client, server)
}
