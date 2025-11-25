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

use crate::nodes::{Node, output, NodeParameter};

pub struct AudioGraph {
    nodes: Vec<Box<dyn Node>>,
    edges: Vec<Vec<usize>>, // TODO (incoming index, message type)
    buffers: Vec<Vec<f32>>,
    message_sender: Sender<AudioGraphMessage>,
    message_receiver: Receiver<AudioGraphMessage>,
    // output_buffer: Vec<f32>,
}

pub enum AudioGraphMessage {
    AddNode(Box<dyn Node>),
    AddEdge((usize, usize)),
}

impl AudioGraph {
    pub fn new() -> Self {
        let output = Box::new(output::Output::new());
        let (message_sender, message_receiver) = channel();
        Self {
            nodes: vec![],
            edges: Vec::new(),
            buffers: Vec::new(),
            message_sender,
            message_receiver,
        }
    }
    pub fn set_root() {
    }
    pub fn process(&mut self, output: &mut [f32]) {
        self.process_messages();
        for node_idx in 0..self.nodes.len() {
            // process all incoming nodes
            for incoming_node_idx in self.edges[node_idx] {
                let inputs: &[&[f32]] = &[];
                self.nodes[incoming_node_idx].process(inputs, self.buffers[incoming_node_idx]);
            }
            // construct inputs array
            self.nodes[node_idx].process(inputs, self.output_buffer);
        }
        zip(self.output_buffer.iter(), output.iter_mut())
            .for_each(|(i, o)| *o += i );
    }
    pub fn get_handle(&self) -> Sender<AudioGraphMessage> {
        self.message_sender.clone()
    }
    fn process_messages(&mut self) {
        for m in self.message_receiver.try_iter() {
            match m {
                AudioGraphMessage::AddNode(node) => {
                    self.nodes.push(node);
                    self.edges.push(Vec::new());
                    self.buffers.push(Vec::new());
                },
                AudioGraphMessage::AddEdge((i, o)) => {
                    self.edges[o].push(i);
                }
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
