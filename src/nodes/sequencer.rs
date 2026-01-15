use std::sync::mpsc::Sender;
use crate::nodes::graph::PortId;
use super::{Node, SAMPLERATE, Message, NodeUi, PortResponses};
use egui::Id;

use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;

#[derive(Debug, Clone)]
pub struct SequencerParameters {
    pub sequence: Vec<f32>,
    pub sender: Sender<Message>,
}

impl SequencerParameters {
    fn handle_message(&mut self, msg: Message) {
        if let Message::Sequence(val) = msg { self.sequence = val }
    }
}

impl NodeUi for SequencerParameters {
    fn draw(&mut self, ctx: &egui::Context, idx: usize) -> PortResponses {
        let mut port_responses = PortResponses::new();
        egui::Window::new("Sequencer").id(Id::new(idx)).show(ctx, |ui| {
            ui.horizontal(|ui| {
                // port buttons
                ui.vertical(|ui| {
                    let sequence_button_response = ui.add(egui::Button::new("trigger"));
                    port_responses.inputs.push(sequence_button_response);
                });
                ui.vertical(|ui| {
                    let mut changed = false;
                    for val in self.sequence.iter_mut() {
                        let val_res = ui.add_enabled(true, egui::DragValue::new(val));
                        if val_res.changed() {
                            changed = true;
                        }
                    }
                    if changed {
                        let _ = self.sender.send(Message::Sequence(self.sequence.clone()));
                    }
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

pub struct SequencerNode {
    params: SequencerParameters,
    idx: usize,
    msg_receiver: Receiver<Message>,
}

impl SequencerNode {
    pub fn new() -> (Self, SequencerParameters) {
        let (msg_sender, msg_receiver) = channel();
        let params = SequencerParameters {
            sequence: vec![220., 275., 220.-55., 330.],
            sender: msg_sender,
        };
        let handler = params.clone();
        (Self {
            params,
            idx: 0,
            msg_receiver,
        }, handler)
    }
}

impl Node for SequencerNode {
    fn process(&mut self, inputs: &[(PortId, &[f32])], output: &mut [f32]) {
        if let Ok(msg) = self.msg_receiver.try_recv() {
            self.params.handle_message(msg);
        }

        let mut idx_buf: Option<Vec<usize>> = None;
        let seq_len = self.params.sequence.len();

        for (port, buffer) in inputs {
            if port.0 == 0 {
                let mut idx = self.idx;
                idx_buf = Some(
                    buffer.iter()
                    .map(|&trigger| {
                        idx = (idx + trigger as usize) % seq_len;
                        idx
                    })
                    .collect()
                );
                self.idx = idx_buf.clone().unwrap().into_iter().last().unwrap();
            }
        }

        for i in 0..output.len() {
            let seq_idx = idx_buf
                .as_ref()
                .map(|b| b[i])
                .unwrap_or(self.idx);

            output[i] = self.params.sequence[seq_idx];
        }
    }
}
