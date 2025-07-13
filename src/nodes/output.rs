// Wasm audio processors can be implemented in Rust without knowing
// about audio worklets.
//
//
// So since the output node is in another thread, its process() will
// call process on nodes that might have been created in another thread
// which means that we need to use some inter-thread syncing
// ie, the UI thread might change parameters, but we can't let that
// happen if the audiocontext is trying to process it

use std::{sync::{Arc, Mutex}};
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
    pub fn process(&self, output: &mut [f32]) -> bool {
        for i in output.iter_mut() {
            *i = 0.0;
        }
        for i in &self.inputs {
            // eh, just copy in for now
            let mut node = i.lock().unwrap();
            // Need to make sure this is true DFS
            // Some kind of 'mark' status in each node?
            let processed_input = node.process(output);
        }
        true
    }
}
