use std::sync::mpsc::Sender;
use crate::nodes::vol_smooth;
use crate::nodes::graph::PortId;

use super::{Node, SAMPLERATE, Message, NodeUi, PortResponses};
use std::f32::consts::PI;
use std::f32::consts::TAU;
use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;

use egui::Id;

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
impl NodeUi for OscParameters {
    fn draw(&mut self, ctx: &egui::Context, idx: usize) -> PortResponses {
        let mut port_responses = PortResponses::new();
        let label = match self.waveform {
            Waveform::Saw => "Sawtooth Oscillator",
            Waveform::Sine => "Sine Oscillator",
            Waveform::Square => "Square Oscillator",
            Waveform::Tri => "Triangle Oscillator",
        };
        egui::Window::new(label).id(Id::new(idx)).show(ctx, |ui| {
            ui.horizontal(|ui| {
                // input ports
                ui.vertical(|ui| {
                    let frequency_button_response = ui.add(egui::Button::new("frequency"));
                    port_responses.inputs.push(frequency_button_response);
                    let volume_button_response = ui.add(egui::Button::new("volume"));
                    port_responses.inputs.push(volume_button_response);
                    let phase_button_response = ui.add(egui::Button::new("phase"));
                    port_responses.inputs.push(phase_button_response);
                });
                // sliders, other non-port UI stuff
                ui.vertical(|ui| {
                    let freq_res = ui.add_enabled(true,
                        egui::Slider::new(&mut self.freq, 0.0..=2000.0)
                        .text("frequency")
                        .logarithmic(true));
                    let vol_res = ui.add_enabled(true,
                        egui::Slider::new(&mut self.target_vol, 0..=100)
                        .text("volume"));
                    if freq_res.changed() {
                        let _ = self.sender.send(Message::Frequency(self.freq));
                    }
                    if vol_res.changed() {
                        let _ = self.sender.send(Message::Volume(self.target_vol));
                    }
                    ui.menu_button("waveform", |ui| {
                        ui.set_width(100.0); // To make sure we wrap long text
                        if ui.button("Sine").clicked() {
                            self.waveform = Waveform::Sine;
                            let _ = self.sender.send(Message::Waveform(Waveform::Sine));
                        }
                        if ui.button("Square").clicked() {
                            self.waveform = Waveform::Square;
                            let _ = self.sender.send(Message::Waveform(Waveform::Square));
                        }
                        if ui.button("Saw").clicked() {
                            self.waveform = Waveform::Saw;
                            let _ = self.sender.send(Message::Waveform(Waveform::Saw));
                        }
                    });
                });
                // output ports
                ui.vertical(|ui| {
                    let out_button_response = ui.add(egui::Button::new("out"));
                    port_responses.outputs.push(out_button_response);
                });
            });
        });
        port_responses
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
    fn process(&mut self, inputs: &[(PortId, &[f32])], output: &mut [f32]) {
        if let Ok(msg) = self.msg_receiver.try_recv() {
            self.params.handle_message(msg);
        };
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
                Waveform::Saw => output[i] = (self.phase - 1.) * (vol as f32 / 100.),
                Waveform::Square => output[i] = self.phase.round() * (vol as f32 / 100.),
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
        let mut output = [0f32; 128];
        sine_osc.process(&[], &mut output);
        assert_eq!(output[0], 0.); // TODO add more cases lol
    }
}
