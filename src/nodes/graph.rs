// things im gonna have to do reguarly:
//   do a dfs/topsort from the root
//      This is prob the part that needs to be fast
//   remove from the graph
//   add to the graph
//
//
// I want it to be owned by the audio thread
// Nodes are created and modified through message passing
//  But the graph itself can modify nodes directly (for parameters/inputs, etc)

use std::sync::mpsc::{channel, Sender, Receiver};
use std::iter::zip;

use crate::nodes::{Node, output};

pub struct AudioGraph {
    nodes: Vec<Box<dyn Node>>,
    edges: Vec<Vec<usize>>,
    message_sender: Sender<AudioGraphMessage>,
    message_receiver: Receiver<AudioGraphMessage>,
    output_buffer: Vec<f32>,
}

pub enum AudioGraphMessage {
    AddNode(Box<dyn Node>),
}

impl AudioGraph {
    pub fn new() -> Self {
        let output = Box::new(output::Output::new());
        let (message_sender, message_receiver) = channel();
        Self {
            nodes: vec![],
            edges: Vec::new(),
            message_sender,
            message_receiver,
            output_buffer: Vec::new(),
        }
    }
    pub fn set_root() {
    }
    pub fn process(&mut self, output: &mut [f32]) {
        self.process_messages();
        for node in self.nodes.iter_mut() {
            self.output_buffer.resize(output.len(), 0.0);
            let fake_input = [0.];
            let inputs: &[&[f32]] = &[&fake_input];
            node.process(inputs, &mut self.output_buffer);
            zip(self.output_buffer.iter(), output.iter_mut())
                .for_each(|(i, o)| *o += i );
        }

    }
    pub fn get_handle(&self) -> Sender<AudioGraphMessage> {
        self.message_sender.clone()
    }
    fn process_messages(&mut self) {
        for m in self.message_receiver.try_iter() {
            match m {
                AudioGraphMessage::AddNode(node) => {
                    self.nodes.push(node);
                },
            }
        }
    }
}

#[cfg(test)]
use wasm_bindgen_test::*;
use crate::nodes::oscillators;
#[cfg(test)]
mod tests {
    use super::*;
    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_add_sine_graph() {
        let mut graph = AudioGraph::new();

        let graph_handler = graph.get_handle();
        let (new_osc, _) = oscillators::SineOsc::new();
        let _ = graph_handler.send(AudioGraphMessage::AddNode(Box::new(new_osc)));
        let mut output = [0f32; 128];
        graph.process(&mut output);
        for sample in output {
            assert_eq!(sample, 0.);
        }
    }
}
