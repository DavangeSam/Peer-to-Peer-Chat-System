//! The `Broadcast` command is responsible for delivering data (a [Bing2BingFrame::Bulk]) to all connected peers.
//! It contains the source of the broadcast, a sequence number, and the data to be broadcasted. 

use super::Command;
use crate::{connection::ConnectionData, parser::Parser, Bing2BingError, Bing2BingFrame};
use async_trait::async_trait;
use bytes::Bytes;
use std::convert::TryFrom;

/// The `Broadcast` command delivers data (a [Bing2BingFrame::Bulk]) to all connected peers.
#[derive(Debug, Clone)]
pub struct Broadcast {
    pub(crate) source: String,
    pub(crate) sequence_number: u64,
    pub(crate) data: Bytes,
}

impl Broadcast {

    /// Creates a new `Broadcast` instance.
    pub fn new(source: String, sequence_number: u64, data: Bytes) -> Self {
        Self {
            source,
            sequence_number,
            data,
        }
    }

    /// Parses frames from the provided parser and constructs a `Broadcast` instance.
    pub(crate) fn parse_frames(parse: &mut Parser) -> Result<Self, Bing2BingError> {
        let source = parse.next_text()?;

        let sequence_number = parse.next_number()?;

        let data = parse.next_bytes()?;
        parse.finish()?;

        Ok(Self {
            source,
            sequence_number,
            data,
        })
    }

    /// Turns this `Broadcast` into a [Bing2BingFrame].
    pub fn into_frame(self) -> Bing2BingFrame {
        // note that using the vec! macro like this is more
        // performant than creating a new vector and then
        // pushing into it according to clippy:
        // https://rust-lang.github.io/rust-clippy/master/index.html#vec_init_then_push
        let cmd = vec![
            Bing2BingFrame::Text("broadcast".to_string()),
            Bing2BingFrame::Text(self.source),
            Bing2BingFrame::Number(self.sequence_number),
            Bing2BingFrame::Bulk(self.data.to_vec()),
        ];

        // cmd.push(Bing2BingFrame::Text("broadcast".to_string()));
        // cmd.push(Bing2BingFrame::Text(self.source));
        // cmd.push(Bing2BingFrame::Number(self.sequence_number));
        // cmd.push(Bing2BingFrame::Bulk(self.data.to_vec()));

        Bing2BingFrame::Array(cmd)
    }
}

/// The `Broadcast` command implements the `Command` trait, which provides methods for getting the sequence number and source,
/// applying the command to connection data, and parsing frames from a parser.
#[async_trait]
impl Command for Broadcast {
    /// Returns the sequence number of the `Broadcast`.
    fn get_sequence_number(&self) -> u64 {
        self.sequence_number
    }

    /// Returns the source of the `Broadcast`.
    fn get_source(&self) -> String {
        self.source.clone()
    }

    /// Forwards this command out to all connected peers.
    async fn apply(self, connection_data: &mut ConnectionData) -> Result<(), Bing2BingError> {
        connection_data
            .peers
            .broadcast(&self.source.clone(), self.into());

        Ok(())
    }

    /// Parses frames from the provided parser and constructs a `Broadcast` instance.
    fn parse_frames(parse: &mut Parser) -> Result<Self, Bing2BingError>
    where
        Self: Sized,
    {
        let source = parse.next_text()?;

        let sequence_number = parse.next_number()?;

        let data = parse.next_bytes()?;
        parse.finish()?;

        Ok(Self {
            source,
            sequence_number,
            data,
        })
    }
}

/// The `Broadcast` command can be converted into a `Bing2BingFrame`.
impl From<Broadcast> for Bing2BingFrame {
    fn from(value: Broadcast) -> Self {
        value.into_frame()
    }
}

/// A `Broadcast` command can be constructed from a `Parser`.
impl TryFrom<&mut Parser> for Broadcast {
    type Error = Bing2BingError;

    fn try_from(value: &mut Parser) -> Result<Self, Self::Error> {
        Self::parse_frames(value)
    }
}
