//! This module contains the implementation of the simple text-based user interface.
//!
//! The simple UI allows users to interact with the application using a set of commands.
//! It communicates with the client and server components using message passing.

use bing2bing_core::Bing2BingFrame;
use bing2bing_core::Client;
use bing2bing_core::ClientServerMessage;
use bing2bing_core::Server;
use chrono::Local;
use chrono::TimeZone;
use chrono::Utc;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;
use tracing::{debug, instrument, trace};

use crate::Cli;

type UiClientTxChannel = mpsc::UnboundedSender<UiClientMessage>;
type UiClientRxChannel = mpsc::UnboundedReceiver<UiClientMessage>;

/// Starts the simple UI with the provided command line arguments.
///
/// # Arguments
///
/// * `args` - The command line arguments.
///
/// # Errors
///
/// Returns a `Bing2BingError` if an error occurs while starting the UI.
pub async fn start(args: Cli) -> Result<(), bing2bing_core::Bing2BingError> {
    // initialize a logger
    tracing_subscriber::fmt::init();

    println!("Starting simple ui with args: {:?}", args);

    let ip_address = args.ip_address.to_string().clone();
    let port = args.port;

    let tracker_ip_address = args.tracker_ip_address.to_string().clone();
    let tracker_port = args.tracker_port;

    let my_name = args.name;

    let max_connections = args.max_connections;

    // *POINTS AVAILABLE*
    // I think this stuff can be refactored to be nicer
    let (ui_client_tx, ui_client_rx) = mpsc::unbounded_channel();

    let (client, server) = bing2bing_core::init(&my_name, &ip_address, port).await;

    let network_client = client.clone();
    std::thread::spawn(move || {
        start_peer(
            my_name,
            network_client,
            server,
            tracker_ip_address,
            tracker_port,
            max_connections,
            ui_client_rx,
        )
    });

    start_ui(App {}, ui_client_tx.clone()).await;

    Ok(())
}

/// Messages that can be sent from the UI to the client.
#[derive(Debug, Clone)]
pub enum UiClientMessage {
    /// Send a chat message.
    Say(String),
    /// Send a private message to a specific user.
    Whisper(String, String),
    /// Ping a user with a specific ID.
    Ping(String, u64),
    /// Send an extension message with an ID and payload.
    Extension(u64, String),
    /// Deliver raw bytes to a specific user.
    Deliver(String, Vec<u8>),
    /// Broadcast raw bytes to all connected users.
    Broadcast(String, Vec<u8>),
}

/// Starts the peer, which includes the client and server components.
///
/// # Arguments
///
/// * `my_name` - The name of the peer.
/// * `client` - The client instance.
/// * `server` - The server instance.
/// * `tracker_ip_address` - The IP address of the tracker server.
/// * `tracker_port` - The port of the tracker server.
/// * `max_incoming_connections` - The maximum number of incoming connections.
/// * `ui_rx` - The receiver channel for messages from the UI.
#[tokio::main]
#[instrument(level = "trace")]
async fn start_peer(
    my_name: String,
    client: Client,
    server: Server,
    tracker_ip_address: String,
    tracker_port: u16,
    max_incoming_connections: u64,
    mut ui_rx: UiClientRxChannel,
) {
    trace!("Starting peer...");
    tokio::spawn(async move {
        server
            .start(
                &tracker_ip_address,
                &tracker_port.to_string(),
                max_incoming_connections,
            )
            .await
            .unwrap_or_else(|e| {
                debug!("Server shut down: {}", e);
            });
    });

    let moved_client = client.clone();

    tokio::spawn(async move {
        loop {
            if let Some(message_from_ui) = ui_rx.recv().await {
                trace!("Received {:?} from Ui", message_from_ui);
                match message_from_ui {
                    UiClientMessage::Whisper(to, message) => {
                        moved_client.whisper(&to, &message).await
                    }
                    UiClientMessage::Say(message) => moved_client.say(&message).await,
                    UiClientMessage::Ping(to, sent_at) => moved_client.ping(&to, sent_at).await,
                    UiClientMessage::Extension(id, payload) => {
                        let payload_frame = Bing2BingFrame::Text(payload);
                        moved_client.send_extension(id, payload_frame).await
                    }
                    UiClientMessage::Deliver(to, data) => {
                        moved_client.deliver(&to, data).await
                    }
                    UiClientMessage::Broadcast(_from, data) => {
                        moved_client.broadcast(data).await
                    }
                }
            }
        }
    });

    let x = tokio::spawn(async move {
        loop {
            trace!("Waiting for next message from client");
            let from_server_message = client.next_message().await;

            let mut stdout = tokio::io::stdout();

            match from_server_message {
                ClientServerMessage::Ping(_) => {
                    panic!("Received a ping message from the server! Shouldn't happen!");
                }
                ClientServerMessage::Pong((from, sent_at)) => {
                    let now = Utc::now();

                    let millis: i64 = sent_at.try_into().unwrap();
                    let then = chrono::Utc.timestamp_millis_opt(millis).single().unwrap();

                    let latency = now.time() - then.time();

                    let formatted_pong = format!(
                        "[{}] PONG response from {}; latency: {}\n",
                        Local::now().format("%Y-%m-%d %H:%M:%S"),
                        from,
                        latency,
                    );
                    stdout.write_all(formatted_pong.as_bytes()).await.unwrap();
                    stdout.flush().await.unwrap();
                }
                ClientServerMessage::Whisper((from, _to, message)) => {
                    let formatted_whisper = format!(
                        "[{}] {} whispered to you: {}\n",
                        Local::now().format("%Y-%m-%d %H:%M:%S"),
                        from,
                        message
                    );
                    stdout
                        .write_all(formatted_whisper.as_bytes())
                        .await
                        .unwrap();
                    stdout.flush().await.unwrap();
                }
                ClientServerMessage::Say((from, message)) => {
                    let formatted_say = format!(
                        "[{}] {}: {}\n",
                        Local::now().format("%Y-%m-%d %H:%M:%S"),
                        from,
                        message
                    );
                    stdout.write_all(formatted_say.as_bytes()).await.unwrap();
                    stdout.flush().await.unwrap();
                }
                ClientServerMessage::Extension((from, id, payload)) => {
                    let formatted_extension = format!(
                        "[{}] {} sent extension {}: {:?}\n",
                        Local::now().format("%Y-%m-%d %H:%M:%S"),
                        from,
                        id,
                        payload
                    );
                    stdout
                        .write_all(formatted_extension.as_bytes())
                        .await
                        .unwrap();
                    stdout.flush().await.unwrap();
                }
                ClientServerMessage::Deliver((from, to, data)) => {
                    let formatted_deliver = format!(
                        "[{}] {} delivered to {}: {:?}\n",
                        Local::now().format("%Y-%m-%d %H:%M:%S"),
                        from,
                        to,
                        data
                    );
                    stdout
                        .write_all(formatted_deliver.as_bytes())
                        .await
                        .unwrap();
                    stdout.flush().await.unwrap();
                }
                ClientServerMessage::Broadcast((from, data)) => {
                    let formatted_broadcast = format!(
                        "[{}] {} broadcasted: {:?}\n",
                        Local::now().format("%Y-%m-%d %H:%M:%S"),
                        from,
                        data
                    );
                    stdout
                        .write_all(formatted_broadcast.as_bytes())
                        .await
                        .unwrap();
                    stdout.flush().await.unwrap();
                }
            }
        }
    });

    x.await.unwrap();
}

/// Starts the user interface.
///
/// # Arguments
///
/// * `app` - The application state.
/// * `client_tx` - The sender channel for messages to the client.
#[instrument(level = "trace")]
async fn start_ui(app: App, client_tx: UiClientTxChannel) {
    let ui_input = tokio::spawn(async move {
        let stdin = std::io::stdin();

        let stdin = stdin.lock();

        for line in std::io::BufRead::lines(stdin) {
            let line = line.unwrap();
            trace!("ui had {:?} entered by user!", line);

            if line == *"/quit" {
                break;
            }

            if line.starts_with("/say ") {
                trace!("input line started with /say !");
                let line = line.clone().strip_prefix("/say ").unwrap().to_string();

                let client_tx = client_tx.clone();

                let msg = UiClientMessage::Say(line);

                trace!("Ui thread sending {:?} over client channle", msg);
                client_tx.send(msg).unwrap();
            } else if line.starts_with("/whisper ") {
                trace!("input line started with /whisper !");
                let line = line.clone().strip_prefix("/whisper ").unwrap().to_string();
                let (to, message) = line.split_once(' ').unwrap();
                let client_tx = client_tx.clone();
                let msg = UiClientMessage::Whisper(to.to_string(), message.to_string());

                trace!("UI thread sending {:?} over client channel", msg);
                client_tx.send(msg).unwrap();

                // the next word should be the destination
            } else if line.starts_with("/ping ") {
                trace!("input line started with /ping !");
                let to = line.clone().strip_prefix("/ping ").unwrap().to_string();
                let client_tx = client_tx.clone();
                let now: u64 = Utc::now().timestamp_millis().try_into().unwrap();
                let msg = UiClientMessage::Ping(to.to_string(), now);

                trace!("UI thread sending {:?} over client channel", msg);
                client_tx.send(msg).unwrap();
            } else if line.starts_with("/extension ") {
                trace!("input line started with /extension !");
                let line = line.strip_prefix("/extension ").unwrap().to_string();
                let (ext_id, payload) = line.split_once(' ').map_or((0, ""), |(id, payload)| {
                    (id.parse::<u64>().unwrap_or(0), payload)
                });
                let client_tx = client_tx.clone();
                let msg = UiClientMessage::Extension(ext_id, payload.to_string());
                trace!("UI thread sending {:?} over client channel", msg);
                client_tx.send(msg).unwrap();
            } else if line.starts_with("/deliver ") {
                trace!("input line started with /deliver !");
                let line = line.strip_prefix("/deliver ").unwrap().to_string();
                let (to, data) = line.split_once(' ').unwrap();
                let data = data.as_bytes().to_vec();
                let client_tx = client_tx.clone();
                let msg = UiClientMessage::Deliver(to.to_string(), data);
                trace!("UI thread sending {:?} over client channel", msg);
                client_tx.send(msg).unwrap();
            } else if line.starts_with("/broadcast ") {
                trace!("input line started with /broadcast !");
                let line = line.strip_prefix("/broadcast ").unwrap().to_string();
                let data = line.as_bytes().to_vec();
                let client_tx = client_tx.clone();
                let msg = UiClientMessage::Broadcast("".to_string(), data);
                trace!("UI thread sending {:?} over client channel", msg);
                client_tx.send(msg).unwrap();
            } else {
                let client_tx = client_tx.clone();
                let msg = UiClientMessage::Say(line);
                trace!("UI thread sending {:?} over client channel", msg);
                client_tx.send(msg).unwrap();
            }
        }
    });

    ui_input.await.unwrap();
}

/// The application state for the simple UI.
#[derive(Debug)]
struct App {}
