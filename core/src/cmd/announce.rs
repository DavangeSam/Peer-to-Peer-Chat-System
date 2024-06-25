use super::Command;
use crate::{
    connection::ConnectionData, peer::PeerData, Bing2BingError, Bing2BingFrame, Parser, Server,
};
use async_trait::async_trait;
use rand::Rng;
use std::{convert::TryFrom, time::Duration};
use tracing::trace;

/// The `Announce` command is propagated through the network to provide peers knowledge about the network topography.
/// I.e., this is how peers let each other know who they are connected to.
#[derive(Debug)]
pub struct Announce {
    pub(crate) source: String,
    pub(crate) sequence_number: u64,
    ip_address: String,
    port: u64,
    available_incoming: u64,
    city: String,
    lat: f64,
    lng: f64,
    peers: Vec<String>,
}

#[async_trait]
impl Command for Announce {
    fn get_sequence_number(&self) -> u64 {
        self.sequence_number
    }

    fn get_source(&self) -> String {
        self.source.clone()
    }

    async fn apply(self, connection_data: &mut ConnectionData) -> Result<(), Bing2BingError> {
        let source = self.source.clone();
        let sequence_number = self.sequence_number;
        let ip_address = self.ip_address.clone();
        let port = self.port;
        let available_incoming = self.available_incoming;
        let city = self.city.clone();
        let lat = self.lat;
        let lng = self.lng;
        let peers = self.peers.clone();

        // add the source's neighbors to our local knowledge
        connection_data.adjacency_list.set(
            self.source.clone(),
            PeerData::new(&city, lat, lng, self.peers.clone()),
            Some(Duration::from_secs(30)),
        );

        // now broadcast the message on to our neigbhors.

        let frame = Self {
            source,
            sequence_number,
            ip_address,
            port,
            available_incoming,
            city,
            lat,
            lng,
            peers,
        }
        .into();

        connection_data.peers.broadcast(&self.source, frame);
        // connection_data.peers

        // Now we want to try to perform an opportunistic connection
        // This helps to ensure that as long as we have at least one
        // outgoing connection, that we should eventually get at least
        // one incoming connection too, which ensures that the
        // there will always be a route to every peer in the network

        // we don't want to connect if we already have an outgoing connection to this
        // peer tho!
        if connection_data.peers.contains_peer(self.source.clone()) {
            return Ok(());
        }

        // we should also flip an `available_incoming` sided coin and
        // connect to this Peer if we get a hit.
        // we also need to force a connection if we don't have _any_ outgoing conncetions.
        // first let's check to see if our current peer has no outgoing connections at all
        let mut available_incoming = available_incoming;

        if connection_data.peers.is_empty() {
            trace!("we had no outgoing connections; but other peer had no available incoming. increasing odds that we will connect to 10%");
            available_incoming = 10;
        }

        let available_range = 0..available_incoming;

        if available_range.is_empty() {
            trace!("peer has no available incoming connecitons and we have at least one outgoing connection");
            return Ok(());
        }

        let mut rng = rand::thread_rng();
        let roll = rng.gen_range(available_range);

        if roll == 0 {
            // EXTRA CREDIT
            // this should be refactored into somewhere else
            // not sure where :3

            // we should go ahead and make an outgoing connection to this peer.
            // let mut peer_map = peer_map;

            // EXTRA CREDIT
            // all these clones are perhaps expensive.
            // there is a better way to deal many of them I suspect
            // especially when it comes to strings vs &str
            let source = self.source.clone();
            let ip_address = self.ip_address.clone();

            Server::connect_to_peer(connection_data.peers, source, ip_address, port.to_string());
        }

        Ok(())
    }

    fn parse_frames(parse: &mut Parser) -> Result<Self, Bing2BingError>
    where
        Self: Sized,
    {
        let source = parse.next_text()?;
        let sequence_number = parse.next_number()?;
        let ip_address = parse.next_text()?;
        let port = parse.next_number()?;
        let available_incoming = parse.next_number()?;

        let city = parse.next_text()?;
        let lat = parse.next_float()?;
        let lng = parse.next_float()?;

        let peers = Announce::parse_peer_info_frames(parse)?;

        parse.finish()?;

        Ok(Self {
            source,
            sequence_number,
            ip_address,
            port,
            available_incoming,
            city,
            lat,
            lng,
            peers,
        })
    }
}

//#[derive(Debug)]
// pub struct AnnounceParams {
//     pub source: String,
//     pub sequence_number: u64,
//     pub ip_address: String,
//     pub port: u64,
//     pub available_incoming: u64,
//     pub city: String,
//     pub lat: f64,
//     pub lng: f64,
//     pub peers: Vec<String>,
// }

impl Announce {
    //pub fn new(params: AnnounceParams) -> Self {
    //     let source = params.source.to_string();
    //     let ip_address = params.ip_address.to_string();
    //     let city = params.city.to_string();
    //     Self {
    //         source: params.source,
    //         sequence_number: params.sequence_number,
    //         ip_address: params.ip_address,
    //         port: params.port,
    //         available_incoming: params.available_incoming,
    //         city: params.city,
    //         lat: params.lat,
    //         lng: params.lng,
    //         peers: params.peers,
    //     }
    // }
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        source: &str,
        sequence_number: u64,
        ip_address: &str,
        port: u64,
        available_incoming: u64,
        city: &str,
        lat: f64,
        lng: f64,
        peers: Vec<String>,
    ) -> Self {
        let source = source.to_string();
        let ip_address = ip_address.to_string();
        let city = city.to_string();
        Self {
            source,
            sequence_number,
            ip_address,
            port,
            available_incoming,
            city,
            lat,
            lng,
            peers,
        }
    }

    // pub(crate) fn parse_frames(
    //     source: String,
    //     sequence_number: u64,
    //     parse: &mut Parse,
    // ) -> Result<Self, Bing2BingError> {

    //     let source = parse.next_text()?;
    //             let sequence_number = parse.next_number()?;
    //     let ip_address = parse.next_text()?;
    //     let port = parse.next_number()?;
    //     let available_incoming = parse.next_number()?;

    //     let city = parse.next_text()?;
    //     let lat = parse.next_float()?;
    //     let lng = parse.next_float()?;

    //     let peers = Announce::parse_peer_info_frames(parse)?;

    //     parse.finish()?;

    //     Ok(Self {
    //         source,
    //         sequence_number,
    //         ip_address,
    //         port,
    //         available_incoming,
    //         city,
    //         lat,
    //         lng,
    //         peers,
    //     })
    // }

    fn parse_peer_info_frames(parse: &mut Parser) -> Result<Vec<String>, Bing2BingError> {
        // This should be an array
        let peer_info_frames = parse.next_array()?;

        // We will loop through each element of the array
        // if it is a Text frame, we will assume that is the name of a peer that
        // the source has an out going connection to.
        let mut ret = vec![];
        for peer_name in peer_info_frames {
            match peer_name {
                Bing2BingFrame::Text(peer_name) => {
                    ret.push(peer_name);
                }
                frame => {
                    return Err(format!(
                    "protocol error; expected text frame when parsing announce peer info, got {:?}",
                    frame
                )
                    .into())
                }
            }
        }

        Ok(ret)
    }

    // pub(crate) async fn my_apply(
    //     &self,
    //     peer_map: &PeerMap,
    //     adjacency_list: &TtlMap<PeerData>,
    // ) -> Result<(), Bing2BingError> {
    //     // let mut peer_map = peer_map.clone();

    //     let source = self.source.clone();
    //     let sequence_number = self.sequence_number;
    //     let ip_address = self.ip_address.clone();
    //     let port = self.port;
    //     let available_incoming = self.available_incoming;
    //     let city = self.city.clone();
    //     let lat = self.lat;
    //     let lng = self.lng;
    //     let peers = self.peers.clone();

    //     // add the source's neighbors to our local knowledge
    //     adjacency_list.set(
    //         self.source.clone(),
    //         PeerData::new(&city, lat, lng, self.peers.clone()),
    //         Some(Duration::from_secs(30)),
    //     );

    //     // now broadcast the message on to our neigbhors.

    //     let frame = Self {
    //         source,
    //         sequence_number,
    //         ip_address,
    //         port,
    //         available_incoming,
    //         city,
    //         lat,
    //         lng,
    //         peers,
    //     }
    //     .into_frame();

    //     peer_map.broadcast(self.source.clone(), frame);

    //     // Now we want to try to perform an opportunistic connection
    //     // This helps to ensure that as long as we have at least one
    //     // outgoing connection, that we should eventually get at least
    //     // one incoming connection too, which ensures that the
    //     // there will always be a route to every peer in the network

    //     // we don't want to connect if we already have an outgoing connection to this
    //     // peer tho!
    //     if peer_map.contains_peer(self.source.clone()) {
    //         return Ok(());
    //     }

    //     // we should also flip an `available_incoming` sided coin and
    //     // connect to this Peer if we get a hit.
    //     let mut rng = rand::thread_rng();
    //     let roll = rng.gen_range(0..available_incoming);

    //     if roll == 0 {
    //         // EXTRA CREDIT
    //         // this should be refactored into somewhere else
    //         // not sure where :3

    //         // we should go ahead and make an outgoing connection to this peer.
    //         // let mut peer_map = peer_map;

    //         // EXTRA CREDIT
    //         // all these clones are perhaps expensive.
    //         // there is a better way to deal many of them I suspect
    //         // especially when it comes to strings vs &str
    //         let source = self.source.clone();
    //         let ip_address = self.ip_address.clone();

    //         Server::connect_to_peer(&peer_map, source, ip_address, port.to_string());
    //     }

    //     Ok(())
    // }

    /// Turns this `Announce` into a [Bing2BingFrame].
    pub fn into_frame(self) -> Bing2BingFrame {
        // note that using the vec! macro like this is more
        // performant than creating a new vector and then
        // pushing into it according to clippy:
        // https://rust-lang.github.io/rust-clippy/master/index.html#vec_init_then_push
        let mut cmd = vec![
            Bing2BingFrame::Text("announce".to_string()),
            Bing2BingFrame::Text(self.source),
            Bing2BingFrame::Number(self.sequence_number),
            Bing2BingFrame::Text(self.ip_address),
            Bing2BingFrame::Number(self.port),
            Bing2BingFrame::Number(self.available_incoming),
            Bing2BingFrame::Text(self.city),
            Bing2BingFrame::Float(self.lat),
            Bing2BingFrame::Float(self.lng),
        ];

        let mut peers = vec![];

        for peer in self.peers {
            peers.push(Bing2BingFrame::Text(peer));
        }

        cmd.push(Bing2BingFrame::Array(peers));

        Bing2BingFrame::Array(cmd)
    }
}

impl From<Announce> for Bing2BingFrame {
    fn from(value: Announce) -> Self {
        value.into_frame()
    }
}

impl TryFrom<&mut Parser> for Announce {
    type Error = Bing2BingError;

    fn try_from(value: &mut Parser) -> Result<Self, Self::Error> {
        Self::parse_frames(value)
    }
}
