// Wasm audio processors can be implemented in Rust without knowing
// about audio worklets.
//

use crate::Node;
use std::iter::zip;

#[allow(dead_code)]
pub struct Output {
    inputs: Vec<Box<dyn Node>>,
    accumulator: u32,
}

#[allow(dead_code)]
impl Output {
    pub fn new() -> Self {
        Self {
            inputs: Vec::new(),
            accumulator: 0,
        }
    }
    pub fn add_input(&mut self, node: Box<dyn Node>) {
        self.inputs.push(node);
    }
}

impl Node for Output {
    fn process(&mut self, inputs: &[&[f32]], output: &mut [f32]) {
        output.iter_mut().for_each( |x| *x = 0.); // TODO is this the best way to zero a chunk?
        for buffer in inputs {
            for (a, b) in zip(output.iter_mut(), buffer.iter()) {
                *a += b;
            }
        }
    }
}
