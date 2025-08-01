// use crate::nodes::Parameter;
use super::{Node, NodeHandle, SAMPLERATE, Parameter};
use std::f32::consts::PI;
use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;

pub enum OscMessage {
    Frequency(f32),
    Volume(u8),
}

#[derive(Default, Debug, Clone)]
pub struct OscParameters {
    pub freq: f32,
    pub vol: u8,
    pub name: String,
}

pub struct SineOsc {
    params: OscParameters,
    phase: f32,
    msg_receiver: Receiver<OscMessage>,
}

impl SineOsc {
    pub fn new(n: &str) -> (Self, NodeHandle) {
        let (msg_sender, msg_receiver) = channel();
        let params = OscParameters {
            freq: 220.,
            vol: 32,
            name: n.to_owned(),
        };
        let handler = NodeHandle {
            sender: msg_sender,
            params: Parameter::Osc(params.clone()), // TODO not tightly coupled...
        };
        (Self {
            params,
            phase: 0.,
            msg_receiver,
        }, handler)
    }
}

impl Node for SineOsc {
    fn process(&mut self, output: &mut [f32]) {
        let msg = self.msg_receiver.try_recv();
        if let Ok(msg) = msg { match msg {
            OscMessage::Frequency(val) => self.params.freq = val as f32,
            OscMessage::Volume(val) => self.params.vol = val,
        } };
        for sample in output.iter_mut() {
            let frequency = self.params.freq;
            let volume = self.params.vol;
            self.phase += 2. * PI / (SAMPLERATE as f32 / frequency);
            self.phase = self.phase.rem_euclid(2. * PI);
            *sample = (self.phase).sin() * (volume as f32 / 100.);
        }
    }
}

pub struct SawtoothOsc {
    params: OscParameters,
    phase: f32,
    msg_receiver: Receiver<OscMessage>,
}

impl SawtoothOsc {
    pub fn new(n: &str) -> (Self, NodeHandle) {
        let (msg_sender, msg_receiver) = channel();
        let params = OscParameters {
            freq: 220.,
            vol: 32,
            name: n.to_owned(),
        };
        let handler = NodeHandle {
            sender: msg_sender,
            params: Parameter::Osc(params.clone()), // TODO not tightly coupled...
        };
        (Self {
            params,
            phase: 0.,
            msg_receiver,
        }, handler)
    }
}

impl Node for SawtoothOsc {
    fn process(&mut self, output: &mut [f32]) {
        let msg = self.msg_receiver.try_recv();
        if let Ok(msg) = msg { match msg {
            OscMessage::Frequency(val) => self.params.freq = val as f32,
            OscMessage::Volume(val) => self.params.vol = val,
        } };
        for a in output {
            let frequency = self.params.freq;
            let volume = self.params.vol;
            self.phase += frequency as f32 / 48_000.;
            self.phase = self.phase.rem_euclid(2.);
            *a = (self.phase - 1.) * (volume as f32 / 100.);
        }
    }
}

pub struct SquareOsc {
    params: OscParameters,
    phase: f32,
    msg_receiver: Receiver<OscMessage>,
}

impl SquareOsc {
    pub fn new(n: &str) -> (Self, NodeHandle) {
        let (msg_sender, msg_receiver) = channel();
        let params = OscParameters {
            freq: 220.,
            vol: 32,
            name: n.to_owned(),
        };
        let handler = NodeHandle {
            sender: msg_sender,
            params: Parameter::Osc(params.clone()),
        };
        (Self {
            params,
            phase: 0.,
            msg_receiver,
        }, handler)
    }
}

impl Node for SquareOsc {
    fn process(&mut self, output: &mut [f32]) {
        let msg = self.msg_receiver.try_recv();
        if let Ok(msg) = msg { match msg {
            OscMessage::Frequency(val) => self.params.freq = val as f32,
            OscMessage::Volume(val) => self.params.vol = val,
        } };
        for a in output {
            let frequency = self.params.freq;
            let volume = self.params.vol;
            self.phase += frequency as f32 / 48_000.;
            self.phase = self.phase.rem_euclid(1.);
            *a = self.phase.round() * (volume as f32 / 100.);
        }
    }
}
