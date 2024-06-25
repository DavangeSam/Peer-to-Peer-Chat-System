use crate::{connection::ConnectionData, util::TtlMap, Bing2BingError, Bing2BingFrame, Parser};
use async_trait::async_trait;
use std::{convert::TryFrom, time::Duration};

mod ping;
pub use ping::Ping;

mod pong;
pub use pong::Pong;

mod say;
pub use say::Say;

mod register;
pub use register::Register;

mod announce;
pub use announce::Announce;

mod whisper;
pub use whisper::Whisper;

mod broadcast;
pub use broadcast::Broadcast;

mod deliver;
pub use deliver::Deliver;

mod extension;
pub use extension::Extension;

#[derive(Debug)]
pub enum Bing2BingCommand {
    Broadcast(Broadcast),
    Ping(Ping),
    Pong(Pong),
    Register(Register),
    Say(Say),
    Deliver(Deliver),
    Announce(Announce),
    Whisper(Whisper),
    Extension(Extension),
    Unknown,
}

impl TryFrom<Bing2BingFrame> for Bing2BingCommand {
    type Error = Bing2BingError;

    fn try_from(value: Bing2BingFrame) -> Result<Self, Self::Error> {
        Bing2BingCommand::from_frame(value)
    }
}

impl Bing2BingCommand {
    pub(crate) fn from_frame(frame: Bing2BingFrame) -> Result<Bing2BingCommand, Bing2BingError> {
        let mut parse = Parser::new(frame)?;

        let command_name = parse.next_text()?.to_lowercase();

        let command = match &command_name[..] {
            "broadcast" => Bing2BingCommand::Broadcast(Broadcast::try_from(&mut parse)?),
            "ping" => Bing2BingCommand::Ping(Ping::try_from(&mut parse)?),
            "pong" => Bing2BingCommand::Pong(Pong::try_from(&mut parse)?),
            "register" => Bing2BingCommand::Register(Register::try_from(&mut parse)?),
            "say" => Bing2BingCommand::Say(Say::try_from(&mut parse)?),
            "deliver" => Bing2BingCommand::Deliver(Deliver::try_from(&mut parse)?),
            "announce" => Bing2BingCommand::Announce(Announce::try_from(&mut parse)?),
            "whisper" => Bing2BingCommand::Whisper(Whisper::try_from(&mut parse)?),
            "extension" => Bing2BingCommand::Extension(Extension::try_from(&mut parse)?),
            _ => return Ok(Bing2BingCommand::Unknown),
        };

        parse.finish()?;

        Ok(command)
    }

    /// POINTS AVAILABLE
    /// There is a way to refactor things such that we could do a call like
    /// `Bing2BingCommand::into_frame(cmd)` instead of having to call
    /// `cmd.into_frame()` directly. This would give us some benefits with
    /// respect to ergonomics (we wouldn't have to have the underlying cmd struct)
    /// fully typed.
    pub fn into_frame(cmd: Bing2BingCommand) -> Bing2BingFrame {
        match cmd {
            Bing2BingCommand::Ping(ping) => ping.into(),
            _ => todo!(),
        }
    }

    /// Checks to make sure that this `Bing2BingCommand` hasn't already been processed.
    /// This helps us ensure that we don't start an infinite loop.
    ///
    /// *THIS IS REALLY UGLY AND COULD BE REFACTORED!*
    pub(crate) fn check_duplicate(&self, processed_commands: &TtlMap<bool>) -> bool {
        let (source, sequence_number) = match self {
            Bing2BingCommand::Announce(announce) => (&announce.source, announce.sequence_number),
            Bing2BingCommand::Broadcast(broadcast) => {
                (&broadcast.source, broadcast.sequence_number)
            }
            Bing2BingCommand::Deliver(deliver) => (&deliver.source, deliver.sequence_number),
            Bing2BingCommand::Register(register) => (&register.peer_name, register.sequence_number),
            Bing2BingCommand::Unknown => return false,
            Bing2BingCommand::Pong(pong) => (&pong.source, pong.sequence_number),
            Bing2BingCommand::Ping(ping) => (&ping.source, ping.sequence_number),
            Bing2BingCommand::Say(say) => (&say.source, say.sequence_number),
            Bing2BingCommand::Whisper(whisper) => (&whisper.source, whisper.sequence_number),
            Bing2BingCommand::Extension(extension) => {
                (&extension.source, extension.sequence_number)
            }
        };

        processed_commands
            .get(&format!("{}-{}", source, sequence_number))
            .is_some()
    }

    /// Marks a command as processed (i.e., it would come back as a duplicate when we call [Self::check_duplicate()])
    pub(crate) fn set_processed(&self, processed_commands: &TtlMap<bool>) {
        let (source, sequence_number) = match self {
            Bing2BingCommand::Announce(announce) => (&announce.source, announce.sequence_number),
            Bing2BingCommand::Broadcast(broadcast) => {
                (&broadcast.source, broadcast.sequence_number)
            }
            Bing2BingCommand::Deliver(deliver) => (&deliver.source, deliver.sequence_number),
            Bing2BingCommand::Register(register) => (&register.peer_name, register.sequence_number),
            Bing2BingCommand::Unknown => return,
            Bing2BingCommand::Ping(ping) => (&ping.source, ping.sequence_number),
            Bing2BingCommand::Pong(pong) => (&pong.source, pong.sequence_number),
            Bing2BingCommand::Say(say) => (&say.source, say.sequence_number),
            Bing2BingCommand::Whisper(whisper) => (&whisper.source, whisper.sequence_number),
            Bing2BingCommand::Extension(extension) => {
                (&extension.source, extension.sequence_number)
            }
        };

        processed_commands.set(
            format!("{}-{}", source, sequence_number),
            true,
            Some(Duration::from_secs(30)),
        );
    }
}

/// A trait defining methods for a protocol level command.
///
/// *THIS IS POORLY NAMED!*
/// There might also be a smarter way to deal with this than using the
/// [`Bing2BingCommand`] enum at all and go full generics everywhere.
#[async_trait]
pub(crate) trait Command {
    /// Returns a parsed command.
    fn parse_frames(parse: &mut Parser) -> Result<Self, Bing2BingError>
    where
        Self: Sized;

    /// Execute the command
    async fn apply(self, connection_data: &mut ConnectionData) -> Result<(), Bing2BingError>;

    /// Get the source of the command
    fn get_source(&self) -> String;

    /// Get the sequence number of the command
    fn get_sequence_number(&self) -> u64;
}
