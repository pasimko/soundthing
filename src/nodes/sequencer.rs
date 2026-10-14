use std::sync::mpsc::Sender;
use crate::nodes::graph::PortId;
use super::{PortInfo, Node, NodeUi, PortDescriptions, drain_messages};

use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;

// Values are laid out four to a row, like the rhythm node's steps
const COLUMNS: usize = 4;
const MAX_ROWS: usize = 16;

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
            vec![
                PortInfo::input("trigger pulse", "Each pulse advances the sequence by one step."),
            ],
            vec![
                PortInfo::output("signal out", "The value of the current step."),
            ]
        )
    }

    fn draw(&mut self, ui: &mut egui::Ui) {
        let mut changed = false;

        ui.horizontal(|ui| {
            ui.label("rows");
            let mut rows = self.sequence.len().div_ceil(COLUMNS).max(1);
            if ui.add(egui::DragValue::new(&mut rows).range(1..=MAX_ROWS)).changed() {
                // New values start at 0
                self.sequence.resize(rows * COLUMNS, 0.0);
                changed = true;
            }
        });

        egui::Grid::new(ui.id().with("values")).show(ui, |ui| {
            for (i, val) in self.sequence.iter_mut().enumerate() {
                if ui.add(egui::DragValue::new(val)).changed() {
                    changed = true;
                }
                if i % COLUMNS == COLUMNS - 1 {
                    ui.end_row();
                }
            }
        });

        if changed {
            let _ = self.sender.send(SequencerMessage::Sequence(self.sequence.clone()));
        }
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
    fn process(&mut self, inputs: &[(PortId, &[f32])], outputs: &mut [Vec<f32>]) {
        let output = &mut outputs[0];
        drain_messages(&self.msg_receiver, |msg| self.params.handle_message(msg));

        let seq_len = self.params.sequence.len();
        if seq_len == 0 {
            output.fill(0.0);
            return;
        }
        // The sequence may have shrunk since the last block
        self.idx %= seq_len;

        let mut idx_buf: Option<Vec<usize>> = None;

        for (port, buffer) in inputs {
            if port.0 == 0 {
                let mut idx = self.idx;
                idx_buf = Some(
                    buffer.iter()
                    // saturating, since a huge trigger value would overflow the addition
                    .map(|&trigger| {
                        idx = idx.saturating_add(trigger as usize) % seq_len;
                        idx
                    })
                    .collect()
                );
                self.idx = idx;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn node(sequence: Vec<f32>) -> SequencerNode {
        let (mut node, _) = SequencerNode::new();
        node.params.sequence = sequence;
        node
    }

    fn run(node: &mut SequencerNode, trigger: &[f32]) -> Vec<f32> {
        let mut outputs = [vec![0.; trigger.len()]];
        node.process(&[(PortId(0), trigger)], &mut outputs);
        let [out] = outputs;
        out
    }

    #[test]
    fn steps_through_any_number_of_values_and_wraps() {
        for len in [1, 3, 4, 8, 13, 64] {
            let sequence: Vec<f32> = (0..len).map(|i| i as f32 * 10.).collect();
            let mut n = node(sequence.clone());
            // a pulse every other sample: the output holds each value until the next pulse
            let trigger: Vec<f32> = (0..(len + 2) * 2).map(|i| if i % 2 == 1 { 1.0 } else { 0.0 }).collect();
            let out = run(&mut n, &trigger);
            let held: Vec<f32> = out.iter().skip(1).step_by(2).copied().collect();
            let expected: Vec<f32> = (1..=len + 2).map(|i| sequence[i % len]).collect();
            assert_eq!(held, expected, "length {len}");
        }
    }

    #[test]
    fn shrinking_the_sequence_never_panics() {
        let mut n = node((0..16).map(|i| i as f32).collect());
        run(&mut n, &[1.0; 13]); // now at step 13
        n.params.sequence = vec![5., 6., 7., 8.];
        // without a trigger input connected, the stale index used to run off the end
        let mut outputs = [vec![0.; 8]];
        n.process(&[], &mut outputs);
        assert!(outputs[0].iter().all(|x| [5., 6., 7., 8.].contains(x)));
        // and with one
        n.params.sequence = vec![1.];
        assert_eq!(run(&mut n, &[1.0, 0.0, 1.0]), vec![1.0, 1.0, 1.0]);
    }

    #[test]
    fn an_empty_sequence_is_silent_not_a_crash() {
        let mut n = node(vec![]);
        assert!(run(&mut n, &[1.0, 1.0]).iter().all(|&x| x == 0.0));
        let mut outputs = [vec![1.0; 4]];
        n.process(&[], &mut outputs);
        assert!(outputs[0].iter().all(|&x| x == 0.0));
    }

    #[test]
    fn huge_trigger_values_dont_overflow() {
        let mut n = node(vec![1., 2., 3.]);
        let out = run(&mut n, &[f32::MAX, f32::INFINITY, 1e30, 1.0]);
        assert!(out.iter().all(|x| [1., 2., 3.].contains(x)));
    }

    #[test]
    fn an_empty_block_doesnt_panic() {
        let mut n = node(vec![1., 2.]);
        assert!(run(&mut n, &[]).is_empty());
    }

    #[test]
    fn the_index_carries_across_blocks() {
        let mut n = node(vec![1., 2., 3., 4.]);
        let mut seen = vec![run(&mut n, &[0.0])[0]];
        for _ in 0..5 {
            seen.push(run(&mut n, &[1.0])[0]);
        }
        assert_eq!(seen, vec![1., 2., 3., 4., 1., 2.]);
    }
}
