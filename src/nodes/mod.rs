use crate::nodes::{
    adsr::{AdsrParameters},
    delay::*,
    oscillators::*,
    // harmonics::*,
    output::*,
    phasor::*,
    graph::{PortId},
    math::*,
    metronome::*,
    noise::*,
    reverb::*,
    sequencer::*,
};

// pub mod timer;
pub mod adsr;
pub mod delay;
pub mod math;
pub mod metronome;
pub mod noise;
pub mod oscillators;
pub mod output;
pub mod phasor;
pub mod reverb;
pub mod sequencer;
// pub mod harmonics;

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
    Adsr(AdsrParameters),
    Delay(DelayParameters),
    Math(MathParameters),
    Metronome(MetronomeParameters),
    Osc(OscParameters),
    Output(OutputParameters),
    Phasor(PhasorParameters),
    Reverb(ReverbParameters),
    Sequencer(SequencerParameters),
    Noise(NoiseParameters),
}

// I don't love this
// TODO split up this god enum?
#[derive(Clone)]
pub enum Message {
    Attack(f32),
    Bpm(f32),
    Damping(u8),
    Decay(f32),
    Delay(u32),
    Frequency(f32),
    Gate(bool),
    Mix(u8),
    Operation(math::Operation),
    Phase(f32),
    Release(f32),
    RoomSize(u8),
    Sequence(Vec<f32>),
    SetA(f32),
    SetB(f32),
    Sustain(f32),
    Volume(u8),
    Waveform(oscillators::Waveform),
}

pub trait NodeUi {
    fn draw(&mut self, ctx: &egui::Context, idx: usize) -> (PortDescriptions, egui::Response);
}

pub struct PortDescriptions {
    pub inputs: Vec<&'static str>,
    pub outputs: Vec<&'static str>,
}

impl PortDescriptions {
    pub fn new() -> Self {
        Self {
            inputs: Vec::new(),
            outputs: Vec::new(),
        }
    }
    pub fn with_ports(inputs: Vec<&'static str>, outputs: Vec<&'static str>) -> Self {
        Self {
            inputs,
            outputs,
        }
    }
}

pub fn vol_smooth(target: f32, start: f32, n: i32) -> f32 {
// TODO check if needs to be f64
    
    target + 0.999_f32.powi(n) * (start - target)
}

fn ratio2pole(t: f32, ratio: f32) -> f32 {
    return ratio.powf(1./((t+f32::EPSILON)*SAMPLERATE as f32));
}
