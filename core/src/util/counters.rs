use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub(crate) struct SequenceNumberGenerator {
    shared: Arc<State>,
}

impl SequenceNumberGenerator {
    pub(crate) fn new(initial_value: u64) -> Self {
        Self {
            shared: Arc::new(State {
                current_number: Mutex::new(initial_value),
            }),
        }
    }

    pub(crate) fn next(&self) -> u64 {
        let mut current_number = self.shared.current_number.lock().unwrap();

        let ret = *current_number;

        *current_number += 1;

        ret
    }
}

#[derive(Debug)]
struct State {
    current_number: Mutex<u64>,
}

#[derive(Debug, Clone)]
pub(crate) struct ConnectionCounter {
    shared: Arc<State>,
}

impl ConnectionCounter {
    pub(crate) fn new(initial_value: u64) -> Self {
        Self {
            shared: Arc::new(State {
                current_number: Mutex::new(initial_value),
            }),
        }
    }

    pub(crate) fn dec(&self) {
        let mut current_number = self.shared.current_number.lock().unwrap();
        *current_number -= 1;
    }

    pub(crate) fn get(&self) -> u64 {
        *self.shared.current_number.lock().unwrap()
    }

    pub(crate) fn inc(&self) {
        let mut current_number = self.shared.current_number.lock().unwrap();
        *current_number += 1;
    }
}
