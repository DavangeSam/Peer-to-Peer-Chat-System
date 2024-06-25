use super::Command;
use crate::{
    connection::ConnectionData, util::TtlMap, Bing2BingError, Bing2BingFrame, Connection, Parser,
};
use async_trait::async_trait;
use std::{convert::TryFrom, net::SocketAddr};
use tracing::{instrument, trace};

/// This command is sent to the tracker to help a peer boostrap itself.
///
/// # Points available
///
/// Right now, [Tracker](crate::Tracker) is a completely separate entity than peers are.
/// There is no reason this has to be the case; it's just easier for boostrapping things.
/// Make it so that your [Server](crate::Server) can also serve as a tracker to help
/// new peers bootstrap things. This should also make the network more resilient to partitions.
#[derive(Debug)]
pub struct Register {
    pub(crate) peer_name: String,
    pub(crate) sequence_number: u64,
    ip_address: String,
    port: String,
}

impl Register {
    pub fn new(peer_name: &str, sequence_number: u64, ip_address: &str, port: &str) -> Self {
        let peer_name = peer_name.to_string();
        let ip_address = ip_address.to_string();
        let port = port.to_string();

        Self {
            peer_name,
            sequence_number,
            ip_address,
            port,
        }
    }

    pub(crate) fn peer_name(&self) -> String {
        self.peer_name.clone()
    }

    #[instrument(level = "trace")]
    pub(crate) async fn apply(
        self,
        known_peers: &TtlMap<SocketAddr>,
        dst: &mut Connection,
    ) -> Result<(), Bing2BingError> {
        trace!("Applying Register command");
        let socket_addr = format!("{}:{}", self.ip_address, self.port).parse::<SocketAddr>()?;

        // see if we already know about this peer name.
        // if we _do_ know, then we then need to check to see if this
        // the peer name is already used or not.
        match known_peers.get(&self.peer_name) {
            Some(addr) => {
                if addr != socket_addr {
                    // this user name is already associated with a different ip/port
                    dst.write_frame(Bing2BingFrame::Error(
                        "Peer name already registered under a different ip:port!".to_string(),
                    ))
                    .await?;
                    return Ok(());
                }
            }
            None => {
                // insert the new entry into the known peers list.
                known_peers.set(self.peer_name, socket_addr, None);
            }
        }

        // construct the random peer list and write it back over the connection
        // the peer list is an array frame that contains arrays that have the name, ip, and port of the peer
        let random_peers = known_peers.random_keys_vals(5);
        let mut frame = vec![];

        random_peers.iter().for_each(|(peer_name, addr)| {
            let ip = addr.ip().to_string();
            let port = addr.port().to_string();

            let peer_info_frame = vec![
                Bing2BingFrame::Text(peer_name.clone()),
                Bing2BingFrame::Text(ip),
                Bing2BingFrame::Text(port),
            ];

            frame.push(Bing2BingFrame::Array(peer_info_frame));
        });

        dst.write_frame(Bing2BingFrame::Array(frame)).await?;

        Ok(())
    }

    /// Turns this `Register` into a [Bing2BingFrame].
    pub fn into_frame(self) -> Bing2BingFrame {
        let cmd = vec![
            Bing2BingFrame::Text("register".to_string()),
            Bing2BingFrame::Text(self.peer_name),
            Bing2BingFrame::Number(self.sequence_number),
            Bing2BingFrame::Text(self.ip_address),
            Bing2BingFrame::Text(self.port),
        ];

        Bing2BingFrame::Array(cmd)
    }
}

#[async_trait]
impl Command for Register {
    fn get_sequence_number(&self) -> u64 {
        self.sequence_number
    }

    fn get_source(&self) -> String {
        self.peer_name.clone()
    }
    async fn apply(self, _connection_data: &mut ConnectionData) -> Result<(), Bing2BingError> {
        todo!()
        // trace!("Applying Register command");
        // let socket_addr = format!("{}:{}", self.ip_address, self.port).parse::<SocketAddr>()?;

        // // see if we already know about this peer name.
        // // if we _do_ know, then we then need to check to see if this
        // // the peer name is already used or not.
        // match known_peers.get(&self.peer_name) {
        //     Some(addr) => {
        //         if addr != socket_addr {
        //             // this user name is already associated with a different ip/port
        //             dst.write_frame(Bing2BingFrame::Error(
        //                 "Peer name already registered under a different ip:port!".to_string(),
        //             ))
        //             .await?;
        //             return Ok(());
        //         }
        //     }
        //     None => {
        //         // insert the new entry into the known peers list.
        //         known_peers.set(self.peer_name, socket_addr, None);
        //     }
        // }

        // // construct the random peer list and write it back over the connection
        // // the peer list is an array frame that contains arrays that have the name, ip, and port of the peer
        // let random_peers = known_peers.random_keys_vals(5);
        // let mut frame = vec![];

        // random_peers.iter().for_each(|(peer_name, addr)| {
        //     let ip = addr.ip().to_string();
        //     let port = addr.port().to_string();

        //     let peer_info_frame = vec![
        //         Bing2BingFrame::Text(peer_name.clone()),
        //         Bing2BingFrame::Text(ip),
        //         Bing2BingFrame::Text(port),
        //     ];

        //     frame.push(Bing2BingFrame::Array(peer_info_frame));
        // });

        // dst.write_frame(Bing2BingFrame::Array(frame)).await?;

        // Ok(())
    }

    fn parse_frames(parse: &mut Parser) -> Result<Self, Bing2BingError>
    where
        Self: Sized,
    {
        let peer_name = parse.next_text()?;

        let sequence_number = parse.next_number()?;

        let ip_address = parse.next_text()?;

        let port = parse.next_text()?;

        Ok(Self {
            peer_name,
            sequence_number,
            ip_address,
            port,
        })
    }
}

impl From<Register> for Bing2BingFrame {
    fn from(value: Register) -> Self {
        value.into_frame()
    }
}

impl TryFrom<&mut Parser> for Register {
    type Error = Bing2BingError;

    fn try_from(value: &mut Parser) -> Result<Self, Self::Error> {
        Self::parse_frames(value)
    }
}
