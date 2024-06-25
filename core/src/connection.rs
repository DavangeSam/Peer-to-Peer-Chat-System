use futures::{SinkExt, StreamExt};

use std::io;
use tokio_serde::formats::SymmetricalJson;

use tokio::net::TcpStream;

use tokio_util::codec::LengthDelimitedCodec;

type Frame = Bing2BingFrame;

use crate::{
    peer::PeerData, peer_map::PeerMap, util::TtlMap, Bing2BingError, Bing2BingFrame, Framed,
};

/// A `Connection` handles reading/writing to the network.
#[derive(Debug)]
pub struct Connection {
    frames: Framed,
}

impl Connection {
    pub async fn new(tcp_stream: TcpStream) -> Connection {
        let length_delimited_frames =
            tokio_util::codec::Framed::new(tcp_stream, LengthDelimitedCodec::new());
        let frames = Framed::new(
            length_delimited_frames,
            SymmetricalJson::<Bing2BingFrame>::default(),
        );

        Connection { frames }
    }

    /// Returns the next [Bing2BingFrame] from the wire.
    pub async fn read_frame(&mut self) -> Result<Option<Frame>, Bing2BingError> {
        match self.frames.next().await {
            Some(Ok(frame)) => Ok(Some(frame)),
            Some(Err(err)) => Err(Box::new(err)),
            None => Ok(None),
        }
    }

    /// Writes a frame to the wire.
    pub async fn write_frame(&mut self, frame: Bing2BingFrame) -> io::Result<()> {
        self.frames.send(frame).await?;

        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct ConnectionData<'a> {
    pub(crate) peers: &'a PeerMap,
    pub(crate) adjacency_list: TtlMap<PeerData>,
    pub(crate) connection: Connection,
}
