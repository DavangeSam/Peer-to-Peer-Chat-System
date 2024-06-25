use rand::prelude::IteratorRandom;
use std::collections::{BTreeMap, HashMap};

use std::sync::Arc;
use std::sync::Mutex;

use std::time::Duration;

use tokio::sync::Notify;

use tokio::time::{self, Instant};

use crate::PeerControlMessage;

/// This is a `HashMap` like structure that uses [String]s as keys,
/// and any value that is [Clone] and [Send](std::marker::Send).
/// The major feature of this struct is that it allows the user to specify an
/// expiration for the key/value pair.
///
/// Large chunks of this code were taken from a similar data structure
/// implemented in [mini-redis](https://github.com/tokio-rs/mini-redis/blob/master/src/db.rs).
/// Be sure to check it out, as it is extremely well documented!.
#[derive(Debug, Clone)]
pub struct TtlMap<T> {
    shared: Arc<Shared<T>>,
}

#[derive(Debug)]
pub struct Shared<T> {
    state: Mutex<State<T>>,
    background_task: Notify,
}

#[derive(Debug)]
struct State<T> {
    entries: HashMap<String, Entry<T>>,

    expirations: BTreeMap<(Instant, u64), String>,

    next_id: u64,
}

#[derive(Debug)]
struct Entry<T> {
    /// Unique id for the entry
    id: u64,

    /// Stored data
    data: T,

    // Instant at which the entry expires and should be removed
    expires_at: Option<Instant>,
}

impl<T: 'static + Clone + std::marker::Send> TtlMap<T> {
    pub(crate) fn new() -> Self {
        let shared = Arc::new(Shared {
            state: Mutex::new(State {
                entries: HashMap::new(),
                expirations: BTreeMap::new(),
                next_id: 0,
            }),
            background_task: Notify::new(),
        });

        tokio::spawn(purge_expired_tasks(shared.clone()));

        Self { shared }
    }

    pub(crate) fn get(&self, key: &str) -> Option<T> {
        let state = self.shared.state.lock().unwrap();
        state.entries.get(key).map(|entry| entry.data.clone())
    }

    pub(crate) fn remove(&mut self, key: &str) -> Option<T> {
        let mut state = self.shared.state.lock().unwrap();

        // Note that we can make use of the fact that [Option] has a map method
        // https://doc.rust-lang.org/stable/std/option/enum.Option.html#method.map
        state.entries.remove(key).map(|entry| entry.data)

        // The above is equivalent to the following:
        // match state.entries.remove(key) {
        //     Some(entry) => Some(entry.data),
        //     None => None,
        // }
    }

    pub(crate) fn set(&self, key: String, value: T, expire: Option<Duration>) {
        let mut state = self.shared.state.lock().unwrap();

        let id = state.next_id;
        state.next_id += 1;

        let mut notify = false;

        let expires_at = expire.map(|duration| {
            let when = Instant::now() + duration;

            notify = state
                .next_expiration()
                .map(|expiration| expiration > when)
                .unwrap_or(true);

            state.expirations.insert((when, id), key.clone());

            when
        });

        let prev = state.entries.insert(
            key,
            Entry {
                id,
                data: value,
                expires_at,
            },
        );

        if let Some(prev) = prev {
            if let Some(when) = prev.expires_at {
                state.expirations.remove(&(when, prev.id));
            }
        }

        drop(state);

        if notify {
            self.shared.background_task.notify_one();
        }
    }

    /// Gets `n` random key/values from this `TtlMap`.
    pub(crate) fn random_keys_vals(&self, n: usize) -> Vec<(String, T)> {
        let state = self.shared.state.lock().unwrap();

        let mut rng = rand::thread_rng();

        // We are going to pull out a random set of keys/values
        let random_keys = state
            .entries
            .keys()
            .choose_multiple(&mut rng, n)
            .to_vec();
            // .iter()
            // .copied()
            // .collect::<Vec<_>>();

        let mut ret = vec![];

        random_keys.iter().for_each(|&key| {
            ret.push((key.clone(), state.entries.get(key).unwrap().data.clone()));
        });

        ret
    }
}

impl<T> Shared<T> {
    /// Purge all expired keys and return the [Instant] that the next key
    /// expires (i.e., when background task should sleep until)
    fn purge_expired_keys(&self) -> Option<Instant> {
        let mut state = self.state.lock().unwrap();

        // TODO: Implement shutdown functionality
        drop(PeerControlMessage::ShutDown);

        // we have to work around the borrow checker here because
        // [Mutex::lock()] returns a [MutexGuard], and the borrow checker
        // can't see through that

        let state = &mut *state;

        // Get all keys that are scheduled to expire before now.
        let now = Instant::now();

        while let Some((&(when, id), key)) = state.expirations.iter().next() {
            if when > now {
                // we are done when "when" is the [Instant] that the next key will expire
                return Some(when);
            }

            // expired key; remove it.
            state.entries.remove(key);
            state.expirations.remove(&(when, id));
        }

        None
    }
}

impl<T> State<T> {
    fn next_expiration(&self) -> Option<Instant> {
        self.expirations
            .keys()
            .next()
            .map(|expiration| expiration.0)
    }
}

async fn purge_expired_tasks<T>(shared: Arc<Shared<T>>) {
    loop {
        if let Some(when) = shared.purge_expired_keys() {
            tokio::select! {
                _ = time::sleep_until(when) => {},
                _ = shared.background_task.notified() => {}
            }
        } else {
            shared.background_task.notified().await;
        }
    }
}
