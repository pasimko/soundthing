use std::sync::mpsc::Sender;
use crate::nodes::graph::PortId;
use super::{Node, SAMPLERATE, NodeUi, PortDescriptions, drain_messages};

use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;

pub enum SequencerMessage {
    Sequence(Vec<f32>),
}

#[derive(Debug, Clone)]
pub struct SequencerParameters {
    pub sequence: Vec<f32>,
    pub sender: Sender<SequencerMessage>,
}

impl SequencerParameters {
    fn handle_message(&mut self, msg: SequencerMessage) {
        match msg {
            SequencerMessage::Sequence(val) => self.sequence = val,
        }
    }
}

impl NodeUi for SequencerParameters {
    fn duplicate(&self) -> Option<(Box<dyn Node>, Box<dyn NodeUi>)> {
        let (node, params) = SequencerNode::from_params(self.clone());
        Some((Box::new(node), Box::new(params)))
    }

    fn title(&self) -> String {
        "Sequencer".to_string()
    }

    fn ports(&self) -> PortDescriptions {
        PortDescriptions::with_ports(
            vec!["trigger pulse"],
            vec!["signal out"]
        )
    }

    fn draw(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                let mut changed = false;
                for val in self.sequence.iter_mut() {
                    let val_res = ui.add_enabled(true, egui::DragValue::new(val));
                    if val_res.changed() {
                        changed = true;
                    }
                }
                if changed {
                    let _ = self.sender.send(SequencerMessage::Sequence(self.sequence.clone()));
                }
            });
        });
    }
}

pub struct SequencerNode {
    params: SequencerParameters,
    idx: usize,
    msg_receiver: Receiver<SequencerMessage>,
}

impl SequencerNode {
    pub fn new() -> (Self, SequencerParameters) {
        let (msg_sender, msg_receiver) = channel();
        let params = SequencerParameters {
            sequence: vec![220., 275., 220.-55., 330.],
            sender: msg_sender,
        };
        Self::build(params, msg_receiver)
    }

    pub fn from_params(mut params: SequencerParameters) -> (Self, SequencerParameters) {
        let (msg_sender, msg_receiver) = channel();
        params.sender = msg_sender;
        Self::build(params, msg_receiver)
    }

    fn build(params: SequencerParameters, msg_receiver: Receiver<SequencerMessage>) -> (Self, SequencerParameters) {
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
        drain_messages(&self.msg_receiver, |msg| self.params.handle_message(msg));

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
