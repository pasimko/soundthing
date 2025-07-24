// Wasm audio processors can be implemented in Rust without knowing
// about audio worklets.
//
//
// So since the output node is in another thread, its process() will
// call process on nodes that might have been created in another thread
// which means that we need to use some inter-thread syncing
// ie, the UI thread might change parameters, but we can't let that
// happen if the audiocontext is trying to process it
//
// So: all our shit needs to be thread-safe + mutex aware

use std::{sync::{Arc, Mutex}, iter::zip};
use super::Node;

// Let's implement a simple sine oscillator with variable frequency and volume.
pub struct Output {
    inputs: Vec<Arc<Mutex<dyn Node>>>,
    muted: bool,
    volume: u8,
    accumulator: u32,
}

impl Output {
    pub fn new() -> Self {
        Self {
            inputs: Vec::new(),
            volume: 255,
            muted: false,
            accumulator: 0,
        }
    }
    pub fn toggle_muted(&mut self) {
        self.muted = !self.muted;
    }
    pub fn set_volume(&mut self, volume: u8) {
        self.volume = volume;
    }
    pub fn attach(&mut self, node: Arc<Mutex<dyn Node>>) {
        self.inputs.push(node);
    }
    pub fn process(&self, buf: &mut [f32]) -> bool {
        for i in buf.iter_mut() {
            *i = 0.0;
        }
        // idk how to tell how big buf is :(
        let mut test : Vec<f32> = Vec::new();
        test.extend_from_slice(buf);
        for (node, i) in zip(&self.inputs, 0..self.inputs.len()) {
            let _ = match node.try_lock() {
                Ok(ref mut node) => node.process(test.as_mut_slice()),
                Err(_) => false,
            };
            // TODO is this slow?
            // and it feels like C
            for i in 0..buf.len() {
                buf[i] += test[i];
            }
            // Need to make sure this is true DFS
            // Some kind of 'mark' status in each node?
            // (if the same node feeds multiple nodes, it'll get process called twice for the same
            // sample)
        }
        true
    }
}
