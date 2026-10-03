// Wasm audio processors can be implemented in Rust without knowing
// about audio worklets.
//

use std::iter::zip;
use crate::{graph, nodes::{PortDescriptions}};
use super::{PortInfo, Node, NodeUi};




#[allow(dead_code)]
pub struct OutputNode {
    inputs: Vec<Box<dyn Node>>,
    accumulator: u32,
}

#[derive(Debug, Clone)]
pub struct OutputParameters {
    pub name: String,
}

impl NodeUi for OutputParameters {
    fn title(&self) -> String {
        self.name.clone()
    }

    fn ports(&self) -> PortDescriptions {
        PortDescriptions::with_ports(
            vec![
                PortInfo::input_many("audio input", "Everything connected here is added together and played."),
            ],
            vec![]
        )
    }

    fn draw(&mut self, _ui: &mut egui::Ui) {}
}

#[allow(dead_code)]
impl OutputNode {
    pub fn new() -> (Self, OutputParameters) {
        let params = OutputParameters {name: "Output".to_string()};
        (Self {
            inputs: Vec::new(),
            accumulator: 0,
        }, params)
    }
    pub fn add_input(&mut self, node: Box<dyn Node>) {
        self.inputs.push(node);
    }
}

impl Node for OutputNode {
    fn process(&mut self, inputs: &[(graph::PortId, &[f32])], outputs: &mut [Vec<f32>]) {
        let output = &mut outputs[0];
        output.iter_mut().for_each( |x| *x = 0.); // TODO is this the best way to zero a chunk?
        for (_, buffer) in inputs {
            for (a, b) in zip(output.iter_mut(), buffer.iter()) {
                *a += b;
            }
        }
    }
}
