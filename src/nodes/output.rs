// Wasm audio processors can be implemented in Rust without knowing
// about audio worklets.

// use std::{clone, sync::atomic::{AtomicBool, AtomicU8, Ordering}};
use super::Node;

// Let's implement a simple sine oscillator with variable frequency and volume.
pub struct Output<'a> {
    inputs: Vec<&'a mut dyn Node>,
    muted: bool,
    volume: u8,
    accumulator: u32,
}

impl<'a> Output<'a> {
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
    pub fn attach(&mut self, node: &'a mut dyn Node) {
        self.inputs.push(node);
    }
}

impl Node for Output<'_> {
    // Add all inputs together
    // TODO divide also
    fn process(&mut self, output: &mut [f32]) -> bool {
        output.iter_mut().map(|_| 0);
        for i in &mut self.inputs {
            // eh, just copy in for now
            let processed_input = i.process(output);
            // output.iter_mut().for_each(f);
        }
        // for a in output {
        //     *a = 0.0;
        // }
        true
    }
}

// #[derive(Default)]
// pub struct OutputParams {
//     // Use atomics for parameters so they can be set in the main thread and
//     // fetched by the audio process thread without further synchronization.
//     inputs: AtoVec<Box<dyn Node>>,
//     muted: AtomicBool,
//     volume: AtomicU8,
// }

// I think I want Parameters to be held within
// these structs
// And then we can just provide getters/setters.
// idk why that's not the model here
