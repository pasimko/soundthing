use std::sync::mpsc::Sender;
use crate::nodes::graph::PortId;
use super::{Node, SAMPLERATE, Message, NodeUi, PortDescriptions};
use egui::Id;

use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;

#[derive(Debug, Clone)]
pub struct NoiseParameters {
    pub target_vol: u8,
    pub sender: Sender<Message>,
}

impl NoiseParameters {
    fn handle_message(&mut self, msg: Message) {
        if let Message::Volume(val) = msg { self.target_vol = val }
    }
}

impl NodeUi for NoiseParameters {
    fn draw(&mut self, ctx: &egui::Context, idx: usize) -> (PortDescriptions, egui::Response) {
        let window = egui::Window::new("Noise").id(Id::new(idx)).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    let vol_res = ui.add_enabled(true,
                        egui::Slider::new(&mut self.target_vol, 0..=100)
                        .text("volume")
                        .logarithmic(true));
                    if vol_res.changed() {
                        let _ = self.sender.send(Message::Volume(self.target_vol));
                    }
                });
            });
        }).unwrap();
        (PortDescriptions::with_ports(
                vec!["volume"],
                vec!["signal out"]
        ), window.response)
    }
}

pub struct NoiseNode {
    params: NoiseParameters,
    state: u32,
    idx: usize,
    msg_receiver: Receiver<Message>,
}

impl NoiseNode {
    pub fn new() -> (Self, NoiseParameters) {
        let (msg_sender, msg_receiver) = channel();
        let params = NoiseParameters {
            target_vol: 100,
            sender: msg_sender,
        };
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
    fn process(&mut self, inputs: &[(PortId, &[f32])], output: &mut [f32]) {
        if let Ok(msg) = self.msg_receiver.try_recv() {
            self.params.handle_message(msg);
        }

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
