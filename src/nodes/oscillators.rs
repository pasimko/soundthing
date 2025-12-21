use std::sync::mpsc::Sender;
use crate::nodes::vol_smooth;
use crate::nodes::graph;

use super::{Node, SAMPLERATE, Message};
use std::f32::consts::PI;
use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;

#[derive(Debug, Clone)]
pub enum Waveform {
    Sine,
    Square,
    Tri,
    Saw,
}

#[derive(Debug, Clone)]
pub struct OscParameters {
    pub freq: f32,
    pub target_vol: u8,
    pub last_vol: f32,
    pub sender: Sender<Message>,
    pub waveform: Waveform,
}

impl OscParameters {
    fn handle_message(&mut self, msg: Message) {
        match msg {
            Message::Frequency(val) => self.freq = val,
            Message::Volume(val) => self.target_vol = val,
            Message::Waveform(val) => self.waveform = val,
            _ => (),
        }
    }
}

pub struct OscNode {
    params: OscParameters,
    phase: f32,
    vol_tick: u32,
    msg_receiver: Receiver<Message>,
}

impl OscNode {
    pub fn new() -> (Self, OscParameters) {
        let (msg_sender, msg_receiver) = channel();
        let params = OscParameters {
            freq: 440.,
            target_vol: 1,
            last_vol: 32. / 100.,
            sender: msg_sender,
            waveform: Waveform::Sine,
        };
        let handler = params.clone();
        (Self {
            params,
            phase: 0.,
            vol_tick: 0,
            msg_receiver,
        }, handler)
    }
}

impl Node for OscNode {
    fn process(&mut self, inputs: &[(graph::PortId, &[f32])], output: &mut [f32]) {
        if let Ok(msg) = self.msg_receiver.try_recv() {
            self.params.handle_message(msg);
        };
        // If we have inputs, use these buffers
        let mut freq_buf = None;
        let mut vol_buf = None;
        for (port, buffer) in inputs {
            // FRAGILE: These have to match the order the ports are declared in the UI
            // A possible future solution will have something like PortType rather than
            // portId, but I am concerned that I might want to have one node with duplicate
            // inputs in the future, so I'm not doing that yet
            match port.0 {
                0 => freq_buf = Some(buffer),
                1 => vol_buf = Some(buffer),
                _ => {}
            }
        }
        for i in 0..output.len() {
            let freq = freq_buf
                .map(|b| b[i])
                .unwrap_or(self.params.freq);

            let vol = vol_buf
                .map(|b| b[i])
                .unwrap_or(self.params.target_vol as f32);

            match self.params.waveform {
                Waveform::Sine => {
                    output[i] = self.phase.sin() * (vol / 100.0);
                    self.phase += 2.0 * PI * freq / SAMPLERATE as f32;
                    self.phase = self.phase.rem_euclid(2.0 * PI);
                },
                Waveform::Saw => {
                    output[i] = (self.phase - 1.) * (vol as f32 / 100.);
                    self.phase += freq / SAMPLERATE as f32;
                    self.phase = self.phase.rem_euclid(2.);
                },
                Waveform::Square => {
                    output[i] = self.phase.round() * (vol as f32 / 100.);
                    self.phase += freq / SAMPLERATE as f32;
                    self.phase = self.phase.rem_euclid(1.);
                }
                Waveform::Tri => {}
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
    fn test_sine() {
        let (mut sine_osc, _) = OscNode::new();
        let mut output = [0f32; 128];
        sine_osc.process(&[], &mut output);
        assert_eq!(output[0], 0.); // TODO add more cases lol
    }
}
