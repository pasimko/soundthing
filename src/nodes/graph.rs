use std::sync::mpsc::{channel, Sender, Receiver};
use std::iter::zip;

use crate::nodes::{Node, output, NodeParameter};

pub struct AudioGraph {
    nodes: Vec<Box<dyn Node>>,
    edges: Vec<Vec<usize>>,
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
    pub fn process(&mut self, output: &mut [f32]) {
        self.process_messages();

        // topologically sort nodes
        let mut visited : Vec<bool> = vec![false; self.nodes.len()];
        let mut top_sorted = Vec::new();

        while let Some(idx) = visited.iter().position(|i| !i) {
            self.visit(idx, &mut visited, &mut top_sorted);
        }

        for node_idx in top_sorted.iter().rev().cloned() {
            // construct inputs array
            let mut inputs: Vec<&[f32]> = Vec::new();
            for incoming_idx in self.edges[node_idx].iter().cloned() {
                inputs.push(self.buffers[incoming_idx].as_slice());
            }
            let mut tmp_output = self.buffers[node_idx].clone();

            self.nodes[node_idx].process(inputs.as_slice(), tmp_output.as_mut_slice());
            self.buffers[node_idx] = tmp_output;
        }
        // TODO keep track of "leaf" OR add output node (and maybe make it invisible)
        if let [_, ..] = self.buffers.as_slice() {
            zip(self.buffers[0].iter(), output.iter_mut())
                .for_each(|(i, o)| *o = *i);
        }
    }
    // return indices, topologically reverse-sorted
    fn visit(&self, node_idx: usize, visited: &mut Vec<bool>, top_sorted: &mut Vec<usize>) {
        if visited[node_idx] {
            return
        }
        visited[node_idx] = true;
        self.edges[node_idx].iter().for_each(|idx| self.visit(idx.clone(), visited, top_sorted));
        top_sorted.push(node_idx);
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
            assert_eq!(sample, 0.); // TODO This make this test normal
        }
    }
}
