use super::Command;
use crate::{connection::ConnectionData, parser::Parser, Bing2BingError, Bing2BingFrame};
use async_trait::async_trait;
use std::convert::TryFrom;
use tracing::{instrument, trace};

/// This command allows for direct messaging between two peers.
/// The idea is that peers should forward this message via the shortest path to the target.
///
/// # Points available.
///
/// Currently, [Whisper::apply()] just treats things as a [Say](crate::cmd::Say), and thus
/// the message is broadcast to all outgoing peers.
/// For extra points, make it only send the data out over the next hop in the shortest path
/// to the destination.
///
/// # Fields
/// * `source`: The sender of the message
/// * `sequence_number`: The sequence number of the message
/// * `destination`: The target recipient of the message
/// * `message`: The content of the message
#[derive(Debug, Clone)]
pub struct Whisper {
    pub(crate) source: String,
    pub(crate) sequence_number: u64,
    pub(crate) destination: String,
    pub(crate) message: String,
}

impl Whisper {
    /// Constructs a new `Whisper` instance.
    /// 
    /// # Arguments
    /// * `source`: The sender of the message
    /// * `sequence_number`: The sequence number of the message
    /// * `destination`: The target recipient of the message
    /// * `message`: The content of the message
    pub fn new(source: &str, sequence_number: u64, destination: &str, message: &str) -> Self {
        let source = source.to_string();
        let destination = destination.to_string();
        let message = message.to_string();

        Self {
            source,
            sequence_number,
            destination,
            message,
        }
    }

    /// Converts the `Whisper` instance into a `Bing2BingFrame`.
    pub fn into_frame(self) -> Bing2BingFrame {
        let cmd = vec![
            Bing2BingFrame::Text("whisper".to_string()),
            Bing2BingFrame::Text(self.source),
            Bing2BingFrame::Number(self.sequence_number),
            Bing2BingFrame::Text(self.destination),
            Bing2BingFrame::Text(self.message),
        ];

        Bing2BingFrame::Array(cmd)
    }
}

#[async_trait]
impl Command for Whisper {
    /// Returns the sequence number of the `Whisper` instance.
    fn get_sequence_number(&self) -> u64 {
        self.sequence_number
    }

    /// Returns the source of the `Whisper` instance.
    fn get_source(&self) -> String {
        self.source.clone()
    }

    /// Currently just broadcasts the message back out to everyone else
    /// This will (eventually) mean that the whisper will arrive at its
    /// destination.
    #[instrument(level = "trace")]
    async fn apply(self, connection_data: &mut ConnectionData) -> Result<(), Bing2BingError> {
        trace!("Applying Whisper command: {:?}", self);

        // connection_data
        //     .peers
        //     .broadcast(&self.source.clone(), self.into());

        // Get the next hop in the shortest path to the destination
        let next_hop = connection_data.adjacency_list.get(&self.destination);

        if let Some(next_hop) = next_hop {
            trace!("Forwarding whisper to next hop: {:?}", next_hop);
            connection_data
                .connection
                .write_frame(self.into_frame())
                .await?;
        } else {
            trace!("No next hop found for destination: {:?}", self.destination);
        }

        Ok(())
    }

    /// Parses the `Whisper` command from a `Parser` instance.
    fn parse_frames(parse: &mut Parser) -> Result<Self, Bing2BingError>
    where
        Self: Sized,
    {
        let source = parse.next_text()?;

        let sequence_number = parse.next_number()?;
        let destination = parse.next_text()?;

        let message = parse.next_text()?;

        parse.finish()?;

        Ok(Self::new(&source, sequence_number, &destination, &message))
    }
}

impl From<Whisper> for Bing2BingFrame {
    /// Converts a `Whisper` instance into a `Bing2BingFrame`.
    fn from(value: Whisper) -> Self {
        value.into_frame()
    }
}

impl TryFrom<&mut Parser> for Whisper {
    type Error = Bing2BingError;

    /// Attempts to convert a `Parser` instance into a `Whisper`.
    fn try_from(value: &mut Parser) -> Result<Self, Self::Error> {
        Self::parse_frames(value)
    }
}
