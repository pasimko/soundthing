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
use std::collections::HashMap;

use crate::nodes::{self, Node, output};

pub struct AudioGraph {
    nodes: Vec<Box<dyn Node>>,
    edges: Vec<Vec<usize>>,
}

impl AudioGraph {
    pub fn new() -> Self {
        let output = Box::new(output::Output::new());
        Self {
            nodes: vec![output],
            edges: Vec::new(),
        }
    }
    pub fn set_root() {
    }
    pub fn process(&mut self, output: &mut [f32]) {
        // Get topological sort of nodes
        // Get all the ones hooked up to the output
        // call process() on each node with the same size as output
        // for each node
        for (i, node) in self.nodes.iter_mut().enumerate() {
            let fake_input = [5.];
            let mut fake_output = [0.];
            let inputs: &[&[f32]] = &[&fake_input];
            node.process(inputs, &mut fake_output);
        }
    }
}
