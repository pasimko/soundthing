use std::sync::mpsc::Sender;
use crate::nodes::graph::PortId;
use super::{PortInfo, Node, SAMPLERATE, NodeUi, PortDescriptions, drain_messages};

use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;

pub enum NoiseMessage {
    Volume(u8),
}

#[derive(Debug, Clone)]
pub struct NoiseParameters {
    pub target_vol: u8,
    pub sender: Sender<NoiseMessage>,
}

impl NoiseParameters {
    fn handle_message(&mut self, msg: NoiseMessage) {
        match msg {
            NoiseMessage::Volume(val) => self.target_vol = val,
        }
    }
}

impl NodeUi for NoiseParameters {
    fn duplicate(&self) -> Option<(Box<dyn Node>, Box<dyn NodeUi>)> {
        let (node, params) = NoiseNode::from_params(self.clone());
        Some((Box::new(node), Box::new(params)))
    }

    fn title(&self) -> String {
        "Noise".to_string()
    }

    fn ports(&self) -> PortDescriptions {
        PortDescriptions::with_ports(
            vec![
                PortInfo::input("volume", "Noise amplitude. Overrides the slider."),
            ],
            vec![
                PortInfo::output("signal out", "White noise."),
            ]
        )
    }

    fn draw(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                let vol_res = ui.add_enabled(true,
                    egui::Slider::new(&mut self.target_vol, 0..=100)
                    .text("volume")
                    .logarithmic(true));
                if vol_res.changed() {
                    let _ = self.sender.send(NoiseMessage::Volume(self.target_vol));
                }
            });
        });
    }
}

pub struct NoiseNode {
    params: NoiseParameters,
    state: u32,
    idx: usize,
    msg_receiver: Receiver<NoiseMessage>,
}

impl NoiseNode {
    pub fn new() -> (Self, NoiseParameters) {
        let (msg_sender, msg_receiver) = channel();
        let params = NoiseParameters {
            target_vol: 100,
            sender: msg_sender,
        };
        Self::build(params, msg_receiver)
    }

    pub fn from_params(mut params: NoiseParameters) -> (Self, NoiseParameters) {
        let (msg_sender, msg_receiver) = channel();
        params.sender = msg_sender;
        Self::build(params, msg_receiver)
    }

    fn build(params: NoiseParameters, msg_receiver: Receiver<NoiseMessage>) -> (Self, NoiseParameters) {
        let handler = params.clone();
        (Self {
            params,
            state: 0xdeadbeef,
            idx: 0,
            msg_receiver,
        }, handler)
    }
}

impl Node for NoiseNode {
    fn process(&mut self, inputs: &[(PortId, &[f32])], outputs: &mut [Vec<f32>]) {
        let output = &mut outputs[0];
        drain_messages(&self.msg_receiver, |msg| self.params.handle_message(msg));

        let mut vol_buf = None;
        for (port, buffer) in inputs {
            // FRAGILE: These have to match the order the ports are declared in the UI
            // A possible future solution will have something like PortType rather than
            // portId, but I am concerned that I might want to have one node with duplicate
            // inputs in the future, so I'm not doing that yet
            match port.0 {
                0 => vol_buf = Some(buffer),
                _ => {}
            }
        }
        for i in 0..output.len() {
            let vol = vol_buf
                .map(|b| b[i])
                .unwrap_or(self.params.target_vol as f32);
            let mut x = self.state;
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            self.state = x;

            output[i] = vol * ((x as f32 / u32::MAX as f32) * 2.0 - 1.0);
        }
    }
}
