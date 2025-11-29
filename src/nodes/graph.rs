use std::sync::mpsc::{channel, Sender, Receiver};
use std::iter::zip;

use crate::nodes::{Node, output};

pub struct AudioGraph {
    nodes: Vec<Box<dyn Node>>,
    sources: Vec<Vec<usize>>,
    message_sender: Sender<AudioGraphMessage>,
    message_receiver: Receiver<AudioGraphMessage>,
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
            nodes: vec![output],
            sources: vec![Vec::new()],
            message_sender,
            message_receiver,
        }
    }
    pub fn process(&mut self, output: &mut [f32]) {
        self.process_messages();
        let mut buffers = vec![];
        for _ in 0..self.nodes.len() {
            buffers.push(vec![0.; output.len()]);
        }

        // topologically sort nodes
        let mut visited : Vec<bool> = vec![false; self.nodes.len()];
        let mut top_sorted = Vec::new();
        while let Some(idx) = visited.iter().position(|i| !i) {
            self.visit(idx, &mut visited, &mut top_sorted);
        }

        for node_idx in top_sorted.iter().cloned() {
            let mut sources: Vec<&[f32]> = Vec::new();
            for source_idx in self.sources[node_idx].iter().cloned() {
                sources.push(buffers[source_idx].as_slice());
            }
            let mut tmp_output = buffers[node_idx].clone();
            self.nodes[node_idx].process(sources.as_slice(), tmp_output.as_mut_slice());
            zip(buffers[node_idx].iter_mut(), tmp_output.iter())
                .for_each(|(o, i)| *o = *i);
        }

        // // TODO keep track of "leaf" OR add output node (and maybe make it invisible)
        zip(buffers[0].iter(), output.iter_mut())
            .for_each(|(i, o)| *o = *i);
    }
    // return indices, topologically reverse-sorted
    fn visit(&self, node_idx: usize, visited: &mut Vec<bool>, top_sorted: &mut Vec<usize>) {
        if visited[node_idx] {
            return
        }
        visited[node_idx] = true;
        self.sources[node_idx].iter().for_each(|idx| self.visit(*idx, visited, top_sorted));
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
                    self.sources.push(Vec::new());
                },
                AudioGraphMessage::AddEdge((source, sink)) => {
                    self.sources[sink].push(source);
                }
            }
        }
    }
}

#[cfg(test)]
use wasm_bindgen_test::*;
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
