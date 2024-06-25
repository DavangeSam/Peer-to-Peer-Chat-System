//! A `Peer` represents an _outbound_ connection.
//! It waits for control messages that come from the server and then
//! deals with them appropriately.
//!
//! Right now, this is very bare bones.
//!
//! It can also likely be _significantly_ refactored and perhaps handled
//! by some other mechanism or data structure that wouldn't require
//! the use of all these channels for communication.

use crate::PeerRxChannel;
use crate::{Bing2BingError, Connection, PeerControlMessage};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tokio::net::TcpStream;

pub(crate) struct Peer {
    info: PeerInfo,
    rx: PeerRxChannel,
}

impl Peer {
    pub(crate) fn new(name: String, ip_address: String, port: String, rx: PeerRxChannel) -> Self {
        let addr: SocketAddr = format!("{}:{}", ip_address, port).parse().unwrap();
        Peer {
            info: PeerInfo { name, addr },
            rx,
        }
    }

    pub(crate) async fn start(&mut self) -> Result<(), Bing2BingError> {
        let tcp_stream = TcpStream::connect(self.info.addr).await.unwrap();

        let mut connection = Connection::new(tcp_stream).await;

        loop {
            tokio::select! {
                Some(control_message) = self.rx.recv() => {
                    // we received something, send it across the network.
                    match control_message {
                        PeerControlMessage::Frame(frame) => {
                            connection.write_frame(frame).await?;
                        },
                        PeerControlMessage::ShutDown => {
                            break;
                        }
                    }

                },
            }
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub(crate) struct PeerInfo {
    name: String,
    addr: SocketAddr,
}

/// POINTS AVAILABLE FOR CLEANING THIS UP (renaming/refactoring as needed?)
/// This is a very poorly named structure that wraps the bits of data
/// that come in over an [Announce](crate::cmd::Announce).
///
/// The [`Default`] implementation generates random lattitude and longitude
/// as well as setting the city to "???".
#[derive(Debug, Clone)]
pub struct PeerData {
    pub city: String,
    pub lat: f64,
    pub lng: f64,
    pub peers: Vec<String>,
}

impl PeerData {
    pub fn new(city: &str, lat: f64, lng: f64, peers: Vec<String>) -> Self {
        Self {
            city: city.to_string(),
            lat,
            lng,
            peers,
        }
    }
}

impl Default for PeerData {
    fn default() -> Self {
        let city = String::from("???");
        // lattitude is on [-90, 90]
        // longitude is on [-180, 180]
        let mut rng = rand::thread_rng();
        let lat = rng.gen_range(-90.0..=90.0);
        let lng = rng.gen_range(-180.0..=180.0);

        Self {
            city,
            lat,
            lng,
            peers: Default::default(),
        }
    }
}
