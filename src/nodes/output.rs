// Wasm audio processors can be implemented in Rust without knowing
// about audio worklets.
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

use std::sync::Arc;
use super::Node;
use ringbuf::{HeapProd, HeapCons, traits::*};

pub struct Output {
    inputs: Vec<HeapCons<f32>>,
    muted: bool,
    volume: u8,
    accumulator: u32,
    output: HeapProd<f32>,
}

impl Output {
    pub fn new(output: HeapProd<f32>) -> Self {
        Self {
            inputs: Vec::new(),
            volume: 255,
            muted: false,
            accumulator: 0,
            output,
        }
    }
    pub fn toggle_muted(&mut self) {
        self.muted = !self.muted;
    }
    pub fn set_volume(&mut self, volume: u8) {
        self.volume = volume;
    }
    pub fn add_input(&mut self, node_output: HeapCons<f32>) {
        self.inputs.push(node_output);
    }
    pub fn process(&mut self) -> bool {

        match self.output.try_push(1.) {
            Ok(_) => true,
            _ => false
        }
    }
}
