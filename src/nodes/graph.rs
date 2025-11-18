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
            nodes: vec![output],
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
        for (i, node) in self.nodes.iter_mut().enumerate() {
            let fake_input = [5.];
            let inputs: &[&[f32]] = &[&fake_input];
            node.process(inputs, output);
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
