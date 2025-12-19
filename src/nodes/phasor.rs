use std::sync::mpsc::Sender;
use super::graph;

use super::{Node, SAMPLERATE, Message};
use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;

#[derive(Debug, Clone)]
pub struct PhasorParameters {
    pub freq: f32,
    pub point: (f32, f32),
    pub sender: Sender<Message>,
    pub name: String,
}

pub struct Phasor {
    params: PhasorParameters,
    phase: f32,
    freq: f32,
    msg_receiver: Receiver<Message>,
    pub point: (f32, f32),
}

impl Phasor {
    pub fn new() -> (Self, PhasorParameters) {
        let (msg_sender, msg_receiver) = channel();
        let point = (0.5, 0.5);
        let freq = 440.;
        let params = PhasorParameters {
            freq,
            sender: msg_sender,
            name: "Phasor".to_string(),
            point,
        };
        let handler = params.clone();
        (Self {
            params,
            freq,
            phase: 0.,
            msg_receiver,
            point,
        }, handler)
    }
}

impl Node for Phasor {
    fn process(&mut self, inputs: &[(graph::Port, &[f32])], output: &mut [f32]) {
        let msg = self.msg_receiver.try_recv();
        if let Ok(msg) = msg { match msg {
            Message::Frequency(val) => self.freq = val,
            Message::Center((x, y)) => self.point = (x, 1.-y),
            _ => (),
        } };
        for sample in output {
            let frequency = self.freq;
            *sample = self.phase;
            if self.phase < self.point.0 {
                self.phase += self.point.1 / self.point.0 / (SAMPLERATE as f32 / frequency);
            }
            else {
                self.phase += (1.-self.point.0) / (1.-self.point.0) / (SAMPLERATE as f32 / frequency);
            }
            self.phase = self.phase.rem_euclid(1.);
        }
    }
}

#[derive(Debug, Clone)]
pub struct PhaselessOscParameters {
    pub target_vol: u8,
    pub last_vol: f32,
    pub sender: Sender<Message>,
    pub name: String,
}

// TODO why did I choose to make Parameters its own
// thing? OR why did I choose to have phase/vol_tick on their own??
pub struct PhaselessSineOsc {
    params: PhaselessOscParameters,
    vol_tick: u32,
    msg_receiver: Receiver<Message>,
    phase: f32,
}

impl PhaselessSineOsc {
    pub fn new() -> (Self, PhaselessOscParameters) {
        let (msg_sender, msg_receiver) = channel();
        let params = PhaselessOscParameters {
            target_vol: 100,
            last_vol: 32. / 100.,
            sender: msg_sender,
            name: "Phaseless Sine Oscillator".to_string(),
        };
        let handler = params.clone();
        (Self {
            params,
            vol_tick: 0,
            msg_receiver,
            phase: 0.,
        }, handler)
    }
}

impl Node for PhaselessSineOsc {
    fn process(&mut self, inputs: &[(graph::Port, &[f32])], output: &mut [f32]) {
        let msg = self.msg_receiver.try_recv();
        if let Ok(msg) = msg { if let Message::Volume(val) = msg {
            self.vol_tick = 0;
            self.params.last_vol = self.params.target_vol as f32 / 100.;
            self.params.target_vol = val;
        } };
        // let volume = vol_smooth(self.params.target_vol as f32, self.params.last_vol, self.vol_tick as i32);
        output.iter_mut().zip(inputs[0].1.iter()).for_each(|(o, &i)| *o = (6.28*i).sin()); // * (self.target_vol as f32 / 100.));
    }
}
