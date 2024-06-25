//! This module contains the implementation of the `Client` struct, which is used to interact with the network.
//!
//! The `Client` struct provides methods for sending various types of messages, such as:
//! - Saying a message that will be broadcast through the network
//! - Whispering a message to a specific user
//! - Pinging another user
//! - Sending an extension message
//! - Delivering a message to another user
//! - Broadcasting a message to all users

use crate::Bing2BingFrame;
use crate::ClientServerMessage;
use crate::{ClientRxChannel, ServerTxChannel};
use std::sync::Arc;
use tracing::{instrument, trace};

/// A `Client` is the way that a user (i.e., a user of our crate) interacts with a [Server](crate::Server),
/// and thus the rest of the network..
#[derive(Debug, Clone)]
pub struct Client {
    shared: Arc<Shared>,
}

impl Client {
    /// Creates a new `Client` instance.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the client.
    /// * `server_tx` - The transmitter channel to send messages to the server.
    /// * `rx` - The receiver channel to receive messages from the server.
    #[instrument(level = "trace")]
    pub fn new(name: String, server_tx: ServerTxChannel, rx: ClientRxChannel) -> Client {
        Self {
            shared: Arc::new(Shared::new(name, server_tx, rx)),
        }
    }

    /// Sends a message that will be broadcast through the network.
    ///
    /// # Arguments
    ///
    /// * `message` - The message to be broadcast.
    #[instrument(level = "trace")]
    pub async fn say(&self, message: &str) {
        let message = ClientServerMessage::Say((self.shared.name.clone(), message.to_string()));

        // Pass the message on to the server
        self.shared.server_tx.send(message).await.unwrap();
    }

    /// Sends a private message to a specific user.
    ///
    /// # Arguments
    ///
    /// * `to` - The name of the user to send the message to.
    /// * `message` - The message to be sent.
    #[instrument(level = "trace")]
    pub async fn whisper(&self, to: &str, message: &str) {
        let message = ClientServerMessage::Whisper((
            self.shared.name.clone(),
            to.to_string(),
            message.to_string(),
        ));

        // pass the message on to the server
        self.shared.server_tx.send(message).await.unwrap();
    }

    /// Pings another user.
    ///
    /// # Arguments
    ///
    /// * `to` - The name of the user to ping.
    /// * `sent_at_millis` - The timestamp (in milliseconds) when the ping was sent.
    #[instrument(level = "trace")]
    pub async fn ping(&self, to: &str, sent_at_millis: u64) {
        let message =
            ClientServerMessage::Ping((self.shared.name.clone(), to.to_string(), sent_at_millis));

        // pass the message on to the server
        self.shared.server_tx.send(message).await.unwrap();
    }

    /// Retrieves the next message that came from the server.
    ///
    /// This method returns an already processed message that the user of the client might be interested in looking at,
    /// such as a message that came from another user and should be displayed in a UI.
    #[instrument(level = "trace")]
    pub async fn next_message(&self) -> ClientServerMessage {
        loop {
            if let Ok(msg) = self.shared.rx.recv().await {
                trace!("Received a ClientServerMessage: {:?}", msg);
                return msg;
            }
        }
    }

    /// Sends an extension message.
    ///
    /// This method is used to extend the functionality of the network.
    ///
    /// # Arguments
    ///
    /// * `extension_id` - The ID that identifies the extension.
    /// * `payload` - The payload of the extension message.
    #[instrument(level = "trace")]
    pub async fn send_extension(&self, extension_id: u64, payload: Bing2BingFrame) {
        let message =
            ClientServerMessage::Extension((self.shared.name.clone(), extension_id, payload));

        // Pass the message on to the server and handle potential errors
        self.shared.server_tx.send(message).await.unwrap();
    }

    /// Delivers a message to another user.
    ///
    /// This method is used to send a message to another user without broadcasting it to the entire network.
    ///
    /// # Arguments
    ///
    /// * `to` - The name of the user that should receive the message.
    /// * `data` - The data to be delivered, as a vector of bytes.
    #[instrument(level = "trace")]
    pub async fn deliver(&self, to: &str, data: Vec<u8>) {
        let message = ClientServerMessage::Deliver((
            self.shared.name.clone(),
            to.to_string(),
            data,
        ));

        // Pass the message
        self.shared.server_tx.send(message).await.unwrap();
    }

    /// Broadcasts a message to all users.
    ///
    /// This method is used to send a message to all users connected to the network.
    ///
    /// # Arguments
    ///
    /// * `data` - The data to be broadcast, as a vector of bytes.
    #[instrument(level = "trace")]
    pub async fn broadcast(&self, data: Vec<u8>) {
        let message = ClientServerMessage::Broadcast((
            self.shared.name.clone(), 
            data.into(),
        ));

        // Pass the message
        self.shared.server_tx.send(message).await.unwrap();
    }

    // /// A method for use by users of `Client` to announce a message to the network.
    // /// This is a way to announce a message to the network.
    // /// The `ip_address` parameter is the ip address of the server.
    // /// The `port` parameter is the port of the server.
    // /// The `available_connections` parameter is the number of available connections.
    // /// The `city` parameter is the city of the server.
    // /// The `latitude` parameter is the latitude of the server.
    // /// The `longitude` parameter is the longitude of the server.
    // /// The `peers` parameter is a vector of peers that the server knows about.
    // #[instrument(level = "trace")]
    // pub async fn announce(
    //     &self,
    //     ip_address: &str,
    //     port: u64,
    //     available_connections: u64,
    //     city: &str,
    //     latitude: f64,
    //     longitude: f64,
    //     peers: Vec<String>,
    // ) {
    //     let message = ClientServerMessage::Announce((
    //         self.shared.name.clone(),
    //         ip_address.to_string(),
    //         port,
    //         available_connections,
    //         city.to_string(),
    //         latitude,
    //         longitude,
    //         peers,
    //     ));

    //     // Pass the message
    //     self.shared.server_tx.send(message).await.unwrap();
    // }
}

/// The shared state of a `Client`.
///
/// This struct contains the data that is shared between the `Client` and its associated `Shared` instance.
#[derive(Debug)]
struct Shared {
    name: String,
    server_tx: ServerTxChannel,
    rx: ClientRxChannel,
}

impl Shared {
    /// Creates a new `Shared` instance.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the client.
    /// * `server_tx` - The transmitter channel to send messages to the server.
    /// * `rx` - The receiver channel to receive messages from the server.
    #[instrument(level = "trace")]
    pub fn new(name: String, server_tx: ServerTxChannel, rx: ClientRxChannel) -> Self {
        Self {
            name,
            server_tx,
            rx,
        }
    }
}
