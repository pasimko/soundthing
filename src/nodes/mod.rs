use crate::nodes::{
    adsr::{AdsrParameters},
    oscillators::*,
    output::*,
    phasor::*,
    graph::{PortId},
    math::*,
    metronome::*,
    sequencer::*,
};

pub mod oscillators;
pub mod adsr;
pub mod metronome;
pub mod sequencer;
// pub mod timer;
pub mod output;
pub mod phasor;
pub mod math;

pub mod graph;

// pub const BUFSIZE: usize = 0x1000;
// pub const SAMPLESIZE: usize = 0x100;
pub const SAMPLERATE: usize = 48_000;

pub trait Node {
    fn process(&mut self, inputs: &[(PortId, &[f32])], output: &mut [f32]);
}

/// Used by UI to display the parameters of a node
// cloned and decoupled from the actual node's parameters
// but message passing should keep them synced.
#[derive(Clone)]
pub enum NodeParameter {
    Osc(OscParameters),
    Phasor(PhasorParameters),
    Adsr(AdsrParameters),
    Output(OutputParameters),
    Math(MathParameters),
    Metronome(MetronomeParameters),
    Sequencer(SequencerParameters),
}

// I don't love this - added because TimerNode needs to be able
// to send a message to any Node and this seemed simplest
// TODO split up this god enum
#[derive(Clone)]
pub enum Message {
    Frequency(f32),
    Phase(f32),
    Gate(bool),
    Volume(u8),
    Waveform(oscillators::Waveform),
    SetA(f32),
    SetB(f32),
    Operation(math::Operation),
    Bpm(f32),
    Sequence(Vec<f32>),
}

pub trait NodeUi {
    fn draw(&mut self, ctx: &egui::Context, idx: usize) -> PortPositions;
}

pub struct PortResponses {
    pub inputs: Vec<egui::Response>,
    pub outputs: Vec<egui::Response>,
}

impl PortResponses {
    pub fn new() -> Self {
        Self {
            inputs: vec!(),
            outputs: vec!(),
        }
    }
}

pub struct PortPositions {
    pub inputs: Vec<egui::Pos2>,
    pub outputs: Vec<egui::Pos2>,
}

impl PortPositions {
    pub fn new() -> Self {
        Self {
            inputs: vec!(),
            outputs: vec!(),
        }
    }
}

pub fn vol_smooth(target: f32, start: f32, n: i32) -> f32 {
// TODO check if needs to be f64
    
    target + 0.999_f32.powi(n) * (start - target)
}

fn ratio2pole(t: f32, ratio: f32) -> f32 {
    return ratio.powf(1./(t*SAMPLERATE as f32));
}
