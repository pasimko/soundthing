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

use crate::Node;
use super::SAMPLESIZE;
use ringbuf::{HeapProd, HeapCons, traits::*};
use std::iter::zip;

pub struct Output {
    inputs: Vec<Box<dyn Node>>,
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
    pub fn add_input(&mut self, node: Box<dyn Node>) {
        self.inputs.push(node);
    }
    // additively mix child nodes
    // need a mechanism to prevent duplicate process calls on ancestors
    pub fn process(&mut self, output: &mut [f32]) {
        output.iter_mut().for_each( |x| *x = 0.);
        for node in &mut self.inputs {
            let mut cur_buf = [0. ; SAMPLESIZE];
            node.process(&mut cur_buf);
            for (a, b) in zip(output.iter_mut(), cur_buf.iter()) {
                *a += b;
            }
        }
    }
}
