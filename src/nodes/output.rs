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
// Hmmm.
// I really need to just bite the bullet and use a ringbuffer...
// 
// Output: Owned by lib, has a thread where it fills ringbuffer
// is rwlock ever the right move?
// (maybe parameters to all the Nodes should be atomic?)
//

// ugh
//

use std::cell::RefCell;
use std::sync::RwLock;
use std::sync::Arc;
use super::Node;

// These need to be mutable
// they're never mutated somewhere else tho...
// hmmm
// how can i avoid a mutex?
// need a way to give readable
pub struct Output {
    inputs: Vec<Arc<dyn Node>>,
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
    pub fn add_node(&mut self, node: Arc<dyn Node>) {
        self.inputs.push(node.clone());
    }
    // need to make sure this doesn't change ownership...
    pub fn get_inputs(&self) -> &[Arc<dyn Node>] {
        self.inputs.as_slice()
    }
    pub fn process(&self, buf: &mut [f32]) -> bool {
        for i in buf.iter_mut() {
            *i = 0.0;
        }
        // idk how to tell how big buf is :(
        // nvm, it's a slice. buf.len() should work
        let mut test : Vec<f32> = Vec::new();
        test.extend_from_slice(buf);
        for node in self.inputs {
            node.process(self.accumulator, test.as_mut_slice());
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
