use super::Command;
use crate::{connection::ConnectionData, Bing2BingError, Bing2BingFrame, Parser};
use async_trait::async_trait;
use std::convert::TryFrom;
use tracing::instrument;

/// A simple command that let's peers test latency between each other.
#[derive(Debug)]
pub struct Ping {
    pub(crate) source: String,
    pub(crate) destination: String,
    pub(crate) sequence_number: u64,
    pub(crate) sent_at: u64,
}

#[async_trait]
impl Command for Ping {
    fn get_sequence_number(&self) -> u64 {
        self.sequence_number
    }

    fn get_source(&self) -> String {
        self.source.clone()
    }

    #[instrument(level = "trace")]
    async fn apply(self, connection_data: &mut ConnectionData) -> Result<(), Bing2BingError> {
        // let response = Bing2BingFrame::Number(self.sequence_number);

        // trace!(?response);

        // connection_data.connection.write_frame(response).await?;
        connection_data
            .peers
            .broadcast(&self.source.clone(), self.into());

        Ok(())
    }

    /// Returns a parsed Ping command.
    fn parse_frames(parse: &mut Parser) -> Result<Self, Bing2BingError>
    where
        Self: Sized,
    {
        let source = parse.next_text()?;

        let destination = parse.next_text()?;

        let sequence_number = parse.next_number()?;

        let sent_at = parse.next_number()?;

        parse.finish()?;

        Ok(Ping::new(&source, &destination, sequence_number, sent_at))
    }
}

impl Ping {
    pub fn new(source: &str, destination: &str, sequence_number: u64, sent_at: u64) -> Self {
        let source = source.to_string();
        let destination = destination.to_string();
        Self {
            source,
            destination,
            sequence_number,
            sent_at,
        }
    }

    /// Turns this `Ping` into a [Bing2BingFrame].
    pub fn into_frame(self) -> Bing2BingFrame {
        // note that using the vec! macro like this is more
        // performant than creating a new vector and then
        // pushing into it according to clippy:
        // https://rust-lang.github.io/rust-clippy/master/index.html#vec_init_then_push
        let cmd = vec![
            Bing2BingFrame::Text("ping".to_string()),
            Bing2BingFrame::Text(self.source),
            Bing2BingFrame::Text(self.destination),
            Bing2BingFrame::Number(self.sequence_number),
            Bing2BingFrame::Number(self.sent_at),
        ];

        // cmd.push(Bing2BingFrame::Text("ping".to_string()));
        // cmd.push(Bing2BingFrame::Text(self.source));
        // cmd.push(Bing2BingFrame::Number(self.sequence_number));

        Bing2BingFrame::Array(cmd)
    }
}

impl From<Ping> for Bing2BingFrame {
    fn from(value: Ping) -> Self {
        value.into_frame()
    }
}

impl TryFrom<&mut Parser> for Ping {
    type Error = Bing2BingError;

    fn try_from(value: &mut Parser) -> Result<Self, Self::Error> {
        Self::parse_frames(value)
    }
}
