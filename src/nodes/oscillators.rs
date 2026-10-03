use std::sync::mpsc::Sender;
use crate::nodes::graph::PortId;

use super::{PortInfo, Node, SAMPLERATE, NodeUi, PortDescriptions, drain_messages};
use std::f32::consts::TAU;
use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;


#[derive(Debug, Clone)]
pub enum Waveform {
    Sine,
    Square,
    Tri,
    Saw,
}

pub enum OscMessage {
    Frequency(f32),
    Volume(u8),
    Waveform(Waveform),
}

#[derive(Debug, Clone)]
pub struct OscParameters {
    pub freq: f32,
    pub target_vol: u8,
    pub last_vol: f32,
    pub sender: Sender<OscMessage>,
    pub waveform: Waveform,
}

impl OscParameters {
    fn handle_message(&mut self, msg: OscMessage) {
        match msg {
            OscMessage::Frequency(val) => self.freq = val,
            OscMessage::Volume(val) => self.target_vol = val,
            OscMessage::Waveform(val) => self.waveform = val,
        }
    }
}
impl NodeUi for OscParameters {
    fn duplicate(&self) -> Option<(Box<dyn Node>, Box<dyn NodeUi>)> {
        let (node, params) = OscNode::from_params(self.clone());
        Some((Box::new(node), Box::new(params)))
    }

    fn title(&self) -> String {
        let label = match self.waveform {
            Waveform::Saw => "Sawtooth Oscillator",
            Waveform::Sine => "Sine Oscillator",
            Waveform::Square => "Square Oscillator",
            Waveform::Tri => "Triangle Oscillator"
        };
        label.to_string()
    }

    fn ports(&self) -> PortDescriptions {
        PortDescriptions::with_ports(
            vec![
                PortInfo::input("frequency", "Pitch in Hz. Overrides the slider."),
                PortInfo::input("volume", "Loudness from 0 to 100. Overrides the slider."),
                PortInfo::input("phase", "Position in the waveform, from 0 to 1. Overrides the oscillator's own phase."),
            ],
            vec![
                PortInfo::output("signal out", "The waveform."),
            ]
        )
    }

    fn draw(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                let freq_res = ui.add_enabled(true,
                    egui::Slider::new(&mut self.freq, 0.001..=2000.0)
                    .text("frequency")
                    .logarithmic(true));
                let vol_res = ui.add_enabled(true,
                    egui::Slider::new(&mut self.target_vol, 0..=100)
                    .text("volume")
                    .logarithmic(true));
                if freq_res.changed() {
                    let _ = self.sender.send(OscMessage::Frequency(self.freq));
                }
                if vol_res.changed() {
                    let _ = self.sender.send(OscMessage::Volume(self.target_vol));
                }
                let waveform_res = ui.menu_button("waveform", |ui| {
                    ui.set_width(100.0); // To make sure we wrap long text
                    if ui.button("Sine").clicked() {
                        self.waveform = Waveform::Sine;
                        let _ = self.sender.send(OscMessage::Waveform(Waveform::Sine));
                    }
                    if ui.button("Square").clicked() {
                        self.waveform = Waveform::Square;
                        let _ = self.sender.send(OscMessage::Waveform(Waveform::Square));
                    }
                    if ui.button("Saw").clicked() {
                        self.waveform = Waveform::Saw;
                        let _ = self.sender.send(OscMessage::Waveform(Waveform::Saw));
                    }
                }).response;
            });
        });
    }
}

pub struct OscNode {
    params: OscParameters,
    phase: f32,
    vol_tick: u32,
    msg_receiver: Receiver<OscMessage>,
}

impl OscNode {
    pub fn new() -> (Self, OscParameters) {
        let (msg_sender, msg_receiver) = channel();
        let params = OscParameters {
            freq: 440.,
            target_vol: 32,
            last_vol: 32. / 100.,
            sender: msg_sender,
            waveform: Waveform::Sine,
        };
        Self::build(params, msg_receiver)
    }

    pub fn from_params(mut params: OscParameters) -> (Self, OscParameters) {
        let (msg_sender, msg_receiver) = channel();
        params.sender = msg_sender;
        Self::build(params, msg_receiver)
    }

    fn build(params: OscParameters, msg_receiver: Receiver<OscMessage>) -> (Self, OscParameters) {
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
    fn process(&mut self, inputs: &[(PortId, &[f32])], outputs: &mut [Vec<f32>]) {
        let output = &mut outputs[0];
        drain_messages(&self.msg_receiver, |msg| self.params.handle_message(msg));
        // If we have inputs, use these buffers
        let mut freq_buf = None;
        let mut vol_buf = None;
        let mut phase_buf = None;
        for (port, buffer) in inputs {
            // FRAGILE: These have to match the order the ports are declared in the UI
            // A possible future solution will have something like PortType rather than
            // portId, but I am concerned that I might want to have one node with duplicate
            // inputs in the future, so I'm not doing that yet
            match port.0 {
                0 => freq_buf = Some(buffer),
                1 => vol_buf = Some(buffer),
                2 => phase_buf = Some(buffer),
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

            let phase = phase_buf
                .map(|b| b[i])
                .unwrap_or( self.phase + freq / SAMPLERATE as f32);

            match self.params.waveform {
                Waveform::Sine => output[i] = (phase * TAU).sin() * (vol / 100.0),
                Waveform::Saw => output[i] = (self.phase - 1.) * (vol / 100.),
                Waveform::Square => output[i] = self.phase.round() * (vol / 100.),
                Waveform::Tri => {}
            }
            self.phase = phase.rem_euclid(1.);
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
        let mut outputs = [vec![0f32; 128]];
        sine_osc.process(&[], &mut outputs);
        assert_eq!(outputs[0][0], 0.); // TODO add more cases lol
    }
}
