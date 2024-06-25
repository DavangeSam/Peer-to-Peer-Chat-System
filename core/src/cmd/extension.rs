use super::Command;
use crate::{
    connection::ConnectionData, parser::Parser, peer_map::PeerMap, Bing2BingError, Bing2BingFrame,
};
use async_trait::async_trait;
use std::{convert::TryFrom, fs::OpenOptions, io::Write};
use tracing::{instrument, trace, warn};
use std::time::{SystemTime, UNIX_EPOCH};

/// This command serves as a mechanism to enable extensions to the protocol.
/// It is esssentially a wrapper around:
///
/// 1. An extension id, which uniquely represents the given extension.
/// 2. A payload [Bing2BingFrame] that is used for whatever the extension is
/// is supposed to do.
///
/// # Points available
///
/// Develop an `Extension`!
#[derive(Debug, Clone)]
pub struct Extension {
    pub(crate) source: String,
    pub(crate) sequence_number: u64,
    pub(crate) extension_id: u64,
    pub(crate) payload: Bing2BingFrame,
}

impl Extension {
    /// Creates a new `Extension` instance.
    pub fn new(
        source: String,
        sequence_number: u64,
        extension_id: u64,
        payload: Bing2BingFrame,
    ) -> Self {
        Self {
            source,
            sequence_number,
            extension_id,
            payload,
        }
    }

    /// This will handle the known and unknown extensions.
    /// It applies the extension command based on the extension id.
    #[instrument(level = "trace")]
    pub(crate) async fn my_apply(&self, peer_map: &PeerMap) -> Result<(), Bing2BingError> {
        trace!("Applying Extension command: {:?}", self);

        match self.extension_id {
            1 => log_system_statistics(self, peer_map).await,
            2 => validate_and_broadcast_message(self, peer_map).await,
            3 => count_connected_peers(self, peer_map).await,
            4 => broadcast_system_time(self, peer_map).await,
            5 => filter_and_forward(self, peer_map).await,
            6 => save_to_file(self, peer_map).await,
            _ => {
                warn!("Unknown extension ID; broadcasting for propagation");

                peer_map.broadcast(&self.source.clone(), self.clone().into_frame());

                Ok(())
            }
        }
    }

    /// Turns this `Extension` into a [Bing2BingFrame].
    pub fn into_frame(self) -> Bing2BingFrame {
        let cmd = vec![
            Bing2BingFrame::Text("extension".to_string()),
            Bing2BingFrame::Text(self.source),
            Bing2BingFrame::Number(self.sequence_number),
            Bing2BingFrame::Number(self.extension_id),
            self.payload,
        ];

        Bing2BingFrame::Array(cmd)
    }
}

#[async_trait]
impl Command for Extension {
    fn get_sequence_number(&self) -> u64 {
        self.sequence_number
    }

    fn get_source(&self) -> String {
        self.source.clone()
    }
    /// this will apply the extension command.
    #[instrument(level = "trace")]
    async fn apply(self, connection_data: &mut ConnectionData) -> Result<(), Bing2BingError> {
        trace!("Applying Extension command: {:?}", self);
        self.my_apply(connection_data.peers).await
    }

    fn parse_frames(parse: &mut Parser) -> Result<Self, Bing2BingError>
    where
        Self: Sized,
    {
        let source = parse.next_text()?;

        let sequence_number = parse.next_number()?;
        let extension_id = parse.next_number()?;

        let payload = parse.next()?;

        parse.finish()?;

        Ok(Self::new(source, sequence_number, extension_id, payload))
    }
}

impl From<Extension> for Bing2BingFrame {
    fn from(value: Extension) -> Self {
        value.into_frame()
    }
}

impl TryFrom<&mut Parser> for Extension {
    type Error = Bing2BingError;

    fn try_from(value: &mut Parser) -> Result<Self, Self::Error> {
        Self::parse_frames(value)
    }
}

/// Extension 1: Log system statistics
/// This extension logs the system statistics to a file named "system_stats.log".
async fn log_system_statistics(
    extension: &Extension,
    peer_map: &PeerMap,
) -> Result<(), Bing2BingError> {
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .append(true)
        .open("system_stats.log")
        .map_err(|e| Bing2BingError::from(e.to_string()))?;

    let log_entry = format!("Stats at {}: {:?}\n", extension.source, extension.payload);
    file.write_all(log_entry.as_bytes())
        .map_err(|e| Bing2BingError::from(e.to_string()))?;
    peer_map.broadcast(&extension.source, extension.clone().into_frame());
    Ok(())
}

/// Extension 2: Validate and broadcast message
/// This extension validates a message and broadcasts it back to all peers while logging the message to a file.
async fn validate_and_broadcast_message(
    extension: &Extension,
    peer_map: &PeerMap,
) -> Result<(), Bing2BingError> {
    if let Bing2BingFrame::Text(msg) = &extension.payload {
        if msg.starts_with("Valid:") {
            let echo_message = format!("Echo from {}: {}", extension.source, msg);
            let frame = Bing2BingFrame::Text(echo_message.clone());
            peer_map.broadcast(&extension.source, frame);

            let mut file = OpenOptions::new()
                .create(true)
                .write(true)
                .append(true)
                .open("validated_messages.log")
                .map_err(|e| Bing2BingError::from(e.to_string()))?;
            let log_entry = format!("Broadcasting a valid message: {}\n", echo_message);
            file.write_all(log_entry.as_bytes())
                .map_err(|e| Bing2BingError::from(e.to_string()))?;
        }
    }
    Ok(())
}

/// Extension 3: Count Connected Peers
/// This extension counts the number of currently connected peers and logs this information.
async fn count_connected_peers(
    extension: &Extension,
    peer_map: &PeerMap,
) -> Result<(), Bing2BingError> {
    let num_peers = peer_map.len();  
    let log_entry = format!("Number of connected peers as reported by {}: {}\n", extension.source, num_peers);
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .append(true)
        .open("peer_count.log")
        .map_err(|e| Bing2BingError::from(e.to_string()))?;
    file.write_all(log_entry.as_bytes())
        .map_err(|e| Bing2BingError::from(e.to_string()))?;
    Ok(())
}


/// Extension 4: Broadcast System Time
/// This extension broadcasts the system's current time to all peers and saves this information to a file.
async fn broadcast_system_time(
    extension: &Extension,
    peer_map: &PeerMap,
) -> Result<(), Bing2BingError> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards");
    let time_message = format!("System time from {}: {}", extension.source, now.as_secs());
    let frame = Bing2BingFrame::Text(time_message.clone());
    peer_map.broadcast(&extension.source, frame);

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .append(true)
        .open("broadcast_times.log")
        .map_err(|e| Bing2BingError::from(e.to_string()))?;
    let log_entry = format!("Broadcasting System Time: {}\n", time_message);
    file.write_all(log_entry.as_bytes())
        .map_err(|e| Bing2BingError::from(e.to_string()))?;
    Ok(())
}

/// Extension 5: Filter and Forward
/// Filters messages for a specific keyword and forwards them if they contain the keyword.
async fn filter_and_forward(
    extension: &Extension,
    peer_map: &PeerMap,
) -> Result<(), Bing2BingError> {
    if let Bing2BingFrame::Text(msg) = &extension.payload {
        if msg.contains("Important") {
            peer_map.broadcast(&extension.source, extension.payload.clone());

            let mut file = OpenOptions::new()
                .create(true)
                .write(true)
                .append(true)
                .open("important_messages.log")
                .map_err(|e| Bing2BingError::from(e.to_string()))?;
            let log_entry = format!("Forwarding important message: {}\n", msg);
            file.write_all(log_entry.as_bytes())
                .map_err(|e| Bing2BingError::from(e.to_string()))?;
        }
    }
    Ok(())
}

/// Extension 6: Save to File
/// Saves incoming messages to a file if they meet a certain criteria.
async fn save_to_file(
    extension: &Extension,
    _peer_map: &PeerMap,
) -> Result<(), Bing2BingError> {
    if let Bing2BingFrame::Text(msg) = &extension.payload {
        if msg.contains("Save") {
            let mut file = OpenOptions::new()
                .create(true)
                .write(true)
                .append(true)
                .open("saved_messages.log")
                .map_err(|e| Bing2BingError::from(e.to_string()))?;
            file.write_all(msg.as_bytes())
                .map_err(|e| Bing2BingError::from(e.to_string()))?;
        }
    }
    Ok(())
}
