use crate::nodes::{adsr::{AdsrParameters}, oscillators::{OscParameters}};

pub mod oscillators;
pub mod adsr;
pub mod output;

// pub const BUFSIZE: usize = 0x1000;
// pub const SAMPLESIZE: usize = 0x100;
pub const SAMPLERATE: usize = 48_000;

// App needs a handler
// handler needs to control params

pub trait Node {
    fn process(&mut self, output: &mut [f32]);
}

// hmm. weird to have as trait *and* enum
pub enum Parameter {
    Osc(OscParameters),
    Adsr(AdsrParameters),
}

// need to make this generic
// return a handle to a node
pub struct NodeHandle {
    pub params: Parameter, // make enum
}
