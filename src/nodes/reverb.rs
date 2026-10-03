use std::sync::mpsc::Sender;
use crate::nodes::graph::PortId;
use super::{PortInfo, Node, SAMPLERATE, NodeUi, PortDescriptions, drain_messages};
use freeverb::Freeverb;

use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;

pub enum ReverbMessage {
    Mix(u8),
    RoomSize(u8),
    Damping(u8),
}

#[derive(Debug, Clone)]
pub struct ReverbParameters {
    pub mix: u8,
    pub room_size: u8,
    pub damping: u8,
    pub sender: Sender<ReverbMessage>,
}

impl ReverbParameters {
    fn handle_message(&mut self, msg: ReverbMessage) {
        match msg {
            ReverbMessage::Mix(val) => self.mix = val,
            ReverbMessage::RoomSize(val) => self.room_size = val,
            ReverbMessage::Damping(val) => self.damping = val,
        }
    }
}

impl NodeUi for ReverbParameters {
    fn duplicate(&self) -> Option<(Box<dyn Node>, Box<dyn NodeUi>)> {
        let (node, params) = ReverbNode::from_params(self.clone());
        Some((Box::new(node), Box::new(params)))
    }

    fn title(&self) -> String {
        "Reverb".to_string()
    }

    fn ports(&self) -> PortDescriptions {
        PortDescriptions::with_ports(
            vec![
                PortInfo::input("signal in", "The signal to add reverb to."),
                PortInfo::input("mix", "Dry/wet balance from 0 to 100. Overrides the slider."),
                PortInfo::input("room size", "Reverb length from 0 to 100. Overrides the slider."),
                PortInfo::input("damping", "How quickly high frequencies fade, from 0 to 100. Overrides the slider."),
            ],
            vec![
                PortInfo::output("signal out", "The dry and reverberated signal, mixed."),
            ]
        )
    }

    fn draw(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_width(100.0);
                let mix_res = ui.add_enabled(true,
                    egui::Slider::new(&mut self.mix, 0..=100)
                    .text("mix")
                );
                let room_res = ui.add_enabled(true,
                    egui::Slider::new(&mut self.room_size, 0..=100)
                    .text("room size")
                );
                let damping_res = ui.add_enabled(true,
                    egui::Slider::new(&mut self.damping, 0..=100)
                    .text("damping")
                );
                if mix_res.changed() {
                    let _ = self.sender.send(ReverbMessage::Mix(self.mix));
                }
                if room_res.changed() {
                    let _ = self.sender.send(ReverbMessage::RoomSize(self.room_size));
                }
                if damping_res.changed() {
                    let _ = self.sender.send(ReverbMessage::Damping(self.damping));
                }
            });
        });
    }
}

pub struct ReverbNode {
    params: ReverbParameters,
    msg_receiver: Receiver<ReverbMessage>,
    freeverb: Freeverb,
    last_room_size: u8,
    last_damping: u8,
}

impl ReverbNode {
    pub fn new() -> (Self, ReverbParameters) {
        let (msg_sender, msg_receiver) = channel();
        let params = ReverbParameters {
            mix: 35,
            room_size: 50,
            damping: 50,
            sender: msg_sender,
        };
        Self::build(params, msg_receiver)
    }

    pub fn from_params(mut params: ReverbParameters) -> (Self, ReverbParameters) {
        let (msg_sender, msg_receiver) = channel();
        params.sender = msg_sender;
        Self::build(params, msg_receiver)
    }

    fn build(params: ReverbParameters, msg_receiver: Receiver<ReverbMessage>) -> (Self, ReverbParameters) {
        let handler = params.clone();
        let mut freeverb = Freeverb::new(SAMPLERATE);
        freeverb.set_room_size(params.room_size as f64 / 100.0);
        freeverb.set_dampening(params.damping as f64 / 100.0);
        let last_room_size = params.room_size;
        let last_damping = params.damping;
        (Self {
            params,
            msg_receiver,
            freeverb,
            last_room_size,
            last_damping,
        }, handler)
    }
}

impl Node for ReverbNode {
    fn process(&mut self, inputs: &[(PortId, &[f32])], output: &mut [f32]) {
        drain_messages(&self.msg_receiver, |msg| self.params.handle_message(msg));
        // If we have inputs, use these buffers
        let mut signal_buf = None;
        let mut mix_buf = None;
        let mut room_buf = None;
        let mut damping_buf = None;
        for (port, buffer) in inputs {
            // FRAGILE: These have to match the order the ports are declared in the UI
            match port.0 {
                0 => signal_buf = Some(buffer),
                1 => mix_buf = Some(buffer),
                2 => room_buf = Some(buffer),
                3 => damping_buf = Some(buffer),
                _ => {}
            }
        }

        if self.params.room_size != self.last_room_size {
            self.freeverb.set_room_size(self.params.room_size as f64 / 100.0);
            self.last_room_size = self.params.room_size;
        }
        if self.params.damping != self.last_damping {
            self.freeverb.set_dampening(self.params.damping as f64 / 100.0);
            self.last_damping = self.params.damping;
        }

        for i in 0..output.len() {
            let signal = signal_buf
                .map(|b| b[i])
                .unwrap_or(0.0);

            let mix = mix_buf
                .map(|b| b[i])
                .unwrap_or(self.params.mix as f32)
                .clamp(0.0, 100.0) / 100.0;

            let room_size = room_buf
                .map(|b| b[i].clamp(0.0, 100.0) / 100.0)
                .unwrap_or(self.params.room_size as f32 / 100.0);
            let damping = damping_buf
                .map(|b| b[i].clamp(0.0, 100.0) / 100.0)
                .unwrap_or(self.params.damping as f32 / 100.0);
            if room_buf.is_some() {
                self.freeverb.set_room_size(room_size as f64);
            }
            if damping_buf.is_some() {
                self.freeverb.set_dampening(damping as f64);
            }

            let (wet_l, wet_r) = self.freeverb.tick((signal as f64, signal as f64));
            let wet = ((wet_l + wet_r) / 2.0) as f32;

            output[i] = signal * (1.0 - mix) + wet * mix;
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
    fn test_reverb_passes_silence() {
        let (mut reverb, _) = ReverbNode::new();
        let mut output = [0f32; 128];
        reverb.process(&[], &mut output);
        assert_eq!(output[0], 0.);
    }
}
