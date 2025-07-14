// Wasm audio processors can be implemented in Rust without knowing
// about audio worklets.

use std::sync::atomic::{AtomicU8, Ordering};
use super::Node;

// Let's implement a simple sine oscillator with variable frequency and volume.
pub struct Oscillator {
    frequency: u8,
    volume: u8,
    accumulator: u32,
}

impl Oscillator {
    pub fn new() -> Self {
        Self {
            frequency: 128,
            volume: 128,
            accumulator: 0,
        }
    }
}

impl Node for Oscillator {
    fn process(&mut self, output: &mut [f32]) -> bool {
        // This method is called in the audio process thread.
        // All imports are set, so host functionality available in worklets
        // (for example, logging) can be used:
        // `web_sys::console::log_1(&JsValue::from(output.len()));`
        // Note that currently TextEncoder and TextDecoder are stubs, so passing
        // strings may not work in this thread.
        for a in output {
            let frequency = self.frequency;
            let volume = self.volume;
            self.accumulator += u32::from(frequency);
            *a = (self.accumulator as f32 / 512.).sin() * (volume as f32 / 100.);
        }
        true
    }
    fn display(&mut self) -> u32 {
        self.frequency.into()
    }
}
