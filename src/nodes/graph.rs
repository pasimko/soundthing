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

use crate::nodes::{Node, output};

pub struct AudioGraph {
    nodes: Vec<Box<dyn Node>>,
    edges: Vec<Vec<usize>>,
    message_sender: Sender<AudioGraphMessage>,
    message_receiver: Receiver<AudioGraphMessage>,
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
        }
    }
    pub fn set_root() {
    }
    pub fn process(&mut self, output: &mut [f32]) {
        // check messages
        // Get topological sort of nodes
        // Get all the ones hooked up to the output
        // call process() on each node with the same size as output
        // for each node
        self.process_messages();
        for node in self.nodes.iter_mut() {
            let fake_input = [0.];
            let inputs: &[&[f32]] = &[&fake_input];
            node.process(inputs, output);
        }
        // This *does* work
        // for (i, sample) in output.iter_mut().enumerate() {
        //     *sample = ((i as f32 * 440.0 * 2.0 * 3.14159 / 48000.0).sin()) * 0.1;
        // }
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
        let (new_osc, _) = oscillators::SineOsc::new("whee");
        let _ = graph_handler.send(AudioGraphMessage::AddNode(Box::new(new_osc)));
        let mut output = [0f32; 128];
        graph.process(&mut output);
        for sample in output {
            assert_eq!(sample, 0.);
        }
    }
}
