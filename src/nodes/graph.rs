use std::sync::mpsc::{channel, Sender, Receiver};
use std::iter::zip;

use crate::nodes::{Node};

// TODO add index trait
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct NodeId(pub usize);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Port(pub u16);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Edge {
    pub from: (NodeId, Port),
    pub to: (NodeId, Port),
}

pub struct AudioGraph {
    nodes: Vec<Box<dyn Node>>,
    incoming_edges: Vec<Vec<Edge>>, // Each node keeps track of its incoming edges
    message_sender: Sender<AudioGraphMessage>,
    message_receiver: Receiver<AudioGraphMessage>,
}

pub enum AudioGraphMessage {
    AddNode(Box<dyn Node>),
    AddEdge(Edge),
}

impl AudioGraph {
    pub fn new() -> Self {
        let (message_sender, message_receiver) = channel();
        Self {
            nodes: vec![],
            incoming_edges: vec![Vec::new()],
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
            let mut sources: Vec<(Port, &[f32])> = Vec::new();
            for edge in self.incoming_edges[node_idx].iter().cloned() {
                let source_idx = edge.from.0;
                let source_port = edge.from.1;
                let sink_port = edge.to.1;
                sources.push((sink_port, buffers[source_idx.0].as_slice()));
            }
            let mut tmp_output = buffers[node_idx].clone();
            self.nodes[node_idx].process(sources.as_slice(), tmp_output.as_mut_slice());
            zip(buffers[node_idx].iter_mut(), tmp_output.iter())
                .for_each(|(o, i)| *o = *i);
        }

        if !buffers.is_empty() {
            zip(buffers[0].iter(), output.iter_mut())
                .for_each(|(i, o)| *o = *i);
        }
    }

    // return indices, topologically sorted
    fn visit(&self, node_idx: usize, visited: &mut Vec<bool>, top_sorted: &mut Vec<usize>) {
        if visited[node_idx] {
            return
        }
        visited[node_idx] = true;
        self.incoming_edges[node_idx].iter().for_each(|edge| self.visit(edge.from.0.0, visited, top_sorted));
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
                    self.incoming_edges.push(Vec::new());
                },
                AudioGraphMessage::AddEdge(edge) => {
                    self.incoming_edges[edge.to.0.0].push(edge); // look at this shit man this is
                                                                 // humiliating you gotta make this
                                                                 // indexable ASAP
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
