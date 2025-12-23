// Wasm audio processors can be implemented in Rust without knowing
// about audio worklets.
//

use std::iter::zip;
use crate::graph;
use super::{Node, SAMPLERATE, Message, NodeUi, PortResponses};
use std::sync::mpsc::Sender;
use crate::nodes::graph::PortId;

use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;

use egui::Id;


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
    fn draw(&mut self, ctx: &egui::Context, idx: usize) -> PortResponses {
        let mut port_responses = PortResponses::new();
        egui::Window::new(self.name.clone()).id(Id::new(idx)).show(ctx, |ui| {
            ui.vertical(|ui| {
                let in_button_response = ui.add(egui::Button::new("in"));
                port_responses.inputs.push(in_button_response);
            });
        });
        port_responses
    }
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
    fn process(&mut self, inputs: &[(graph::PortId, &[f32])], output: &mut [f32]) {
        output.iter_mut().for_each( |x| *x = 0.); // TODO is this the best way to zero a chunk?
        for (_, buffer) in inputs {
            for (a, b) in zip(output.iter_mut(), buffer.iter()) {
                *a += b;
            }
        }
    }
}
