use std::sync::mpsc::Sender;
use crate::nodes::PortDescriptions;
use crate::nodes::graph::PortId;
use super::{PortInfo, Node, SAMPLERATE, NodeUi, drain_messages};

use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;

#[derive(Debug, Clone)]
pub enum Operation {
    Add,
    Mul,
    Exp,
}

pub enum MetronomeMessage {
    Bpm(f32),
}

#[derive(Debug, Clone)]
pub struct MetronomeParameters {
    pub bpm: f32,
    pub sender: Sender<MetronomeMessage>,
}

impl MetronomeParameters {
    fn handle_message(&mut self, msg: MetronomeMessage) {
        match msg {
            MetronomeMessage::Bpm(val) => self.bpm = val,
        }
    }
}

impl NodeUi for MetronomeParameters {
    fn duplicate(&self) -> Option<(Box<dyn Node>, Box<dyn NodeUi>)> {
        let (node, params) = MetronomeNode::from_params(self.clone());
        Some((Box::new(node), Box::new(params)))
    }

    fn title(&self) -> String {
        "Metronome".to_string()
    }

    fn ports(&self) -> PortDescriptions {
        PortDescriptions::with_ports(
            vec![
                PortInfo::input("bpm", "Tempo in beats per minute. Overrides the slider."),
            ],
            vec![
                PortInfo::output("pulse out", "1 for a single sample on each beat, otherwise 0."),
            ]
        )
    }

    fn draw(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_width(100.0);
                let bpm_res = ui.add_enabled(true,
                    egui::Slider::new(&mut self.bpm, 0.1..=2000.0)
                    .text("bpm")
                    .logarithmic(true)
                );
                if bpm_res.changed() {
                    let _ = self.sender.send(MetronomeMessage::Bpm(self.bpm));
                }
            });
        });
    }
}

pub struct MetronomeNode {
    params: MetronomeParameters,
    phase: f32,
    msg_receiver: Receiver<MetronomeMessage>,
}

impl MetronomeNode {
    pub fn new() -> (Self, MetronomeParameters) {
        let (msg_sender, msg_receiver) = channel();
        let params = MetronomeParameters {
            bpm: 120.0,
            sender: msg_sender,
        };
        Self::build(params, msg_receiver)
    }

    pub fn from_params(mut params: MetronomeParameters) -> (Self, MetronomeParameters) {
        let (msg_sender, msg_receiver) = channel();
        params.sender = msg_sender;
        Self::build(params, msg_receiver)
    }

    fn build(params: MetronomeParameters, msg_receiver: Receiver<MetronomeMessage>) -> (Self, MetronomeParameters) {
        let handler = params.clone();
        (Self {
            params,
            phase: 0.0,
            msg_receiver,
        }, handler)
    }
}

impl Node for MetronomeNode {
    fn process(&mut self, inputs: &[(PortId, &[f32])], output: &mut [f32]) {
        drain_messages(&self.msg_receiver, |msg| self.params.handle_message(msg));
        // If we have inputs, use these buffers
        let mut bpm_buf = None;
        for (port, buffer) in inputs {
            // FRAGILE: These have to match the order the ports are declared in the UI
            if port.0 == 0 { bpm_buf = Some(buffer) }
        }
        for i in 0..output.len() {
            let bpm = bpm_buf
                .map(|b| b[i])
                .unwrap_or(self.params.bpm);

            self.phase += bpm / SAMPLERATE as f32 / 60.;

            // this is wasm we branch in this muthafucka
            // better take yo sensitive ass back to x86
            if self.phase >= 1.0 {
                output[i] = 1.0;
                self.phase = self.phase.rem_euclid(1.);
            }
            else {
                output[i] = 0.0;
            }
        }
    }
}
