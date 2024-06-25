use crate::Bing2BingFrame;
use crate::{PeerControlMessage, PeerTxChannel};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use tracing::debug;
use tracing::{error, instrument, trace};

/// A `PeerMap` contains data and functionality related to peers that
/// this peer has initiated connections with.
/// I.e., the peers that this peer can send comands _to_.
///
/// It is worth taking a look at this implementation (as well as [TtlMap](crate::util::TtlMap)).
/// It makes use of a relatively common pattern of wrapping a [Mutex] in an [Arc] in a way
/// that makes it easier to handle locks.
///
/// The major difficulty it is addressing is that when you acquire a lock to a [Mutex],
/// it stays locked until the [Mutex] is goes out of scope.
/// This has consequences when you try to lock something and then use await without
/// the lock going out of scope.
///
/// You can, of course force the dropping of the lock. For example:
/// ```
/// let mut mutex = Mutex::new(10);
/// let x = 0;
///
/// {
///   let mutex = mutex.lock.unwrap();
///   x = *mutex;
///   *mutex += 1;
/// }
///
/// some_function().await;
/// ```
/// If you don't include the explicit lifetime scope with the `{` `}`, then the
/// call to `await` will keep the mutex locked until it finishes
#[derive(Debug, Clone)]
pub struct PeerMap {
    // peers: HashMap<SocketAddr, ClientTxChannel>,
    shared: Arc<Shared>,
}

#[derive(Debug)]
struct Shared {
    state: Mutex<State>,
}

#[derive(Debug)]
struct State {
    entries: HashMap<String, PeerTxChannel>,
}

impl Default for PeerMap {
    fn default() -> Self {
        Self::new()
    }
}

impl PeerMap {
    pub fn new() -> Self {
        let shared = Arc::new(Shared {
            state: Mutex::new(State {
                entries: HashMap::new(),
            }),
        });

        PeerMap { shared }
    }

    /// Does this `PeerMap` contain a peer with the given name?
    pub(crate) fn contains_peer(&self, name: String) -> bool {
        let state = self.shared.state.lock().unwrap();
        state.entries.contains_key(&name)
    }

    /// Get the number of peers we have an outgoing connection to
    pub(crate) fn len(&self) -> usize {
        let state = self.shared.state.lock().unwrap();
        state.entries.len()
    }

    /// Whether or not the peer map is empty
    pub(crate) fn is_empty(&self) -> bool {
        let state = self.shared.state.lock().unwrap();
        state.entries.is_empty()
    }

    /// Returns a list of the peers that are currently in this `PeerMap`.
    pub(crate) fn peer_names(&self) -> Vec<String> {
        let state = self.shared.state.lock().unwrap();
        state.entries.keys().cloned().collect::<Vec<_>>()
    }

    /// Insert a new peer
    pub(crate) fn insert(
        &mut self,
        peer_name: String,
        peer_tx: PeerTxChannel,
    ) -> Option<PeerTxChannel> {
        let mut state = self.shared.state.lock().unwrap();

        // we need to create a new Peer and get it into  our peer map
        state.entries.insert(peer_name, peer_tx)
    }

    /// Removes a peer from this `PeerMap`.
    pub(crate) fn remove(&mut self, peer_name: String) -> Option<PeerTxChannel> {
        let mut state = self.shared.state.lock().unwrap();
        state.entries.remove(&peer_name)
    }

    /// Broadcasts a message to every one of our peers (out links)
    /// Idk if this needs to be async?
    #[instrument(level = "trace")]
    pub fn broadcast(&self, sender: &str, frame: Bing2BingFrame) {
        debug!("broadcasting frame: {:?}", frame);
        let mut state = self.shared.state.lock().unwrap();

        for (peer_name, peer_tx) in state.entries.iter_mut() {
            if *peer_name != sender {
                // we are sending this message to another Client struct
                // over the channel.
                // this allows for cross task communication, just like a std::mpsc does.
                trace!(
                    "Broadcasting frame \"{:?}\" to client {:?}",
                    frame,
                    *peer_name
                );

                let frame = PeerControlMessage::Frame(frame.clone());
                if let Err(err) = peer_tx.send(frame) {
                    error!(
                        "There was an error when trying to broadcast to peer {:?}: {:?}",
                        *peer_name, err
                    );
                };
            }
        }
    }
}
