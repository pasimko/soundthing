use std::sync::mpsc::Sender;
use crate::nodes::graph::PortId;
use super::{PortInfo, Node, NodeUi, PortDescriptions, drain_messages};

use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;

// Steps are read left to right, then top to bottom, so each row is one bar of four beats
const COLUMNS: usize = 4;
const MAX_ROWS: usize = 16;

const CELL_SIZE: f32 = 22.0;
const CELL_GAP: f32 = 4.0;

pub enum RhythmMessage {
    Steps(Vec<bool>),
}

#[derive(Debug, Clone)]
pub struct RhythmParameters {
    pub steps: Vec<bool>,
    pub sender: Sender<RhythmMessage>,
}

impl RhythmParameters {
    fn handle_message(&mut self, msg: RhythmMessage) {
        match msg {
            RhythmMessage::Steps(val) => self.steps = val,
        }
    }
}

impl NodeUi for RhythmParameters {
    fn duplicate(&self) -> Option<(Box<dyn Node>, Box<dyn NodeUi>)> {
        let (node, params) = RhythmNode::from_params(self.clone());
        Some((Box::new(node), Box::new(params)))
    }

    fn title(&self) -> String {
        "Rhythm".to_string()
    }

    fn ports(&self) -> PortDescriptions {
        PortDescriptions::with_ports(
            vec![
                PortInfo::input("trigger", "Each time this goes from zero to nonzero, the node plays its current step and moves on to the next."),
            ],
            vec![
                PortInfo::output("pulse out", "1 for a single sample if the step just played is checked, otherwise 0."),
            ]
        )
    }

    fn draw(&mut self, ui: &mut egui::Ui) {
        let mut changed = false;

        ui.horizontal(|ui| {
            ui.label("rows");
            let mut rows = self.steps.len() / COLUMNS;
            if ui.add(egui::DragValue::new(&mut rows).range(1..=MAX_ROWS)).changed() {
                self.steps.resize(rows * COLUMNS, false);
                changed = true;
            }
        });

        let rows = self.steps.len() / COLUMNS;
        let grid_size = egui::vec2(
            COLUMNS as f32 * (CELL_SIZE + CELL_GAP) - CELL_GAP,
            rows as f32 * (CELL_SIZE + CELL_GAP) - CELL_GAP,
        );
        let (grid, _) = ui.allocate_exact_size(grid_size, egui::Sense::hover());
        for (i, step) in self.steps.iter_mut().enumerate() {
            let (row, column) = (i / COLUMNS, i % COLUMNS);
            let cell = egui::Rect::from_min_size(
                grid.min + egui::vec2(column as f32, row as f32) * (CELL_SIZE + CELL_GAP),
                egui::Vec2::splat(CELL_SIZE),
            );
            let response = ui.interact(cell, ui.id().with(("step", i)), egui::Sense::click());
            if response.clicked() {
                *step = !*step;
                changed = true;
            }
            let fill = if *step {
                egui::Color32::from_rgb(235, 235, 225)
            } else {
                egui::Color32::from_rgb(45, 45, 55)
            };
            let outline = if response.hovered() { egui::Color32::WHITE } else { egui::Color32::BLACK };
            ui.painter().rect_filled(cell, 2.0, fill);
            ui.painter().rect_stroke(cell, 2.0, egui::Stroke::new(1.0, outline), egui::StrokeKind::Inside);
        }

        if changed {
            let _ = self.sender.send(RhythmMessage::Steps(self.steps.clone()));
        }
    }
}

pub struct RhythmNode {
    params: RhythmParameters,
    active: usize, // the step the next trigger plays
    trigger_high: bool, // was the trigger nonzero on the previous sample?
    msg_receiver: Receiver<RhythmMessage>,
}

impl RhythmNode {
    pub fn new() -> (Self, RhythmParameters) {
        let (msg_sender, msg_receiver) = channel();
        let params = RhythmParameters {
            steps: [true, false, false, false].repeat(2),
            sender: msg_sender,
        };
        Self::build(params, msg_receiver)
    }

    pub fn from_params(mut params: RhythmParameters) -> (Self, RhythmParameters) {
        let (msg_sender, msg_receiver) = channel();
        params.sender = msg_sender;
        Self::build(params, msg_receiver)
    }

    fn build(params: RhythmParameters, msg_receiver: Receiver<RhythmMessage>) -> (Self, RhythmParameters) {
        let handler = params.clone();
        (Self {
            params,
            active: 0,
            trigger_high: false,
            msg_receiver,
        }, handler)
    }
}

impl Node for RhythmNode {
    fn process(&mut self, inputs: &[(PortId, &[f32])], outputs: &mut [Vec<f32>]) {
        let output = &mut outputs[0];
        drain_messages(&self.msg_receiver, |msg| self.params.handle_message(msg));
        // If we have inputs, use these buffers
        let mut trigger_buf = None;
        for (port, buffer) in inputs {
            // FRAGILE: These have to match the order the ports are declared in the UI
            if port.0 == 0 { trigger_buf = Some(buffer) }
        }
        for i in 0..output.len() {
            let trigger = trigger_buf
                .map(|b| b[i])
                .unwrap_or(0.0);

            // Only the sample where the trigger goes nonzero counts, so a trigger held high
            // (or stretched by a pulsewidth node) plays one step rather than racing through them
            let high = trigger.abs() > 0.0;
            output[i] = 0.0;
            let steps = self.params.steps.len();
            if high && !self.trigger_high && steps > 0 {
                // the pattern may have shrunk since the last trigger
                self.active %= steps;
                if self.params.steps[self.active] {
                    output[i] = 1.0;
                }
                self.active = (self.active + 1) % steps;
            }
            self.trigger_high = high;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(node: &mut RhythmNode, trigger: &[f32]) -> Vec<f32> {
        let mut outputs = [vec![0.; trigger.len()]];
        node.process(&[(PortId(0), trigger)], &mut outputs);
        let [out] = outputs;
        out
    }

    fn pulses(n: usize) -> Vec<f32> {
        // a one-sample pulse every other sample
        (0..n * 2).map(|i| if i % 2 == 0 { 1.0 } else { 0.0 }).collect()
    }

    #[test]
    fn plays_the_pattern_in_order_and_wraps() {
        let (mut node, _) = RhythmNode::new();
        node.params.steps = vec![true, false, true, true, false];
        let out = run(&mut node, &pulses(11));
        let played: Vec<f32> = out.iter().step_by(2).copied().collect();
        assert_eq!(played, vec![1., 0., 1., 1., 0., 1., 0., 1., 1., 0., 1.]);
        // and nothing between the triggers
        assert!(out.iter().skip(1).step_by(2).all(|&x| x == 0.0));
    }

    #[test]
    fn a_held_trigger_advances_once() {
        let (mut node, _) = RhythmNode::new();
        node.params.steps = vec![true, false, true, true];
        let mut trigger = vec![1.0; 100];
        trigger.extend(vec![0.0; 100]);
        trigger.extend(vec![-0.5; 100]); // any nonzero value counts
        let out = run(&mut node, &trigger);
        // step 0 (checked) plays on the first trigger; the later ones play step 1, which is unchecked
        assert_eq!(out.iter().filter(|&&x| x == 1.0).count(), 1);
        assert_eq!(out[0], 1.0);
        assert_eq!(out[200], 0.0);
    }

    #[test]
    fn state_carries_across_blocks() {
        let (mut node, _) = RhythmNode::new();
        node.params.steps = vec![true, false, true, false];
        let mut played = Vec::new();
        for _ in 0..4 {
            played.push(run(&mut node, &[1.0, 0.0, 0.0])[0]);
        }
        assert_eq!(played, vec![1., 0., 1., 0.]);
    }

    #[test]
    fn shrinking_the_pattern_never_panics() {
        let (mut node, _) = RhythmNode::new();
        node.params.steps = vec![true; 16];
        run(&mut node, &pulses(10)); // active is now 10
        node.params.steps = vec![true, false, false, false];
        let out = run(&mut node, &pulses(3));
        assert!(out.iter().filter(|&&x| x == 1.0).count() <= 3);
        node.params.steps = vec![];
        assert!(run(&mut node, &pulses(3)).iter().all(|&x| x == 0.0));
    }

    #[test]
    fn nothing_connected_is_silent() {
        let (mut node, _) = RhythmNode::new();
        let mut outputs = [vec![1.0; 64]];
        node.process(&[], &mut outputs);
        assert!(outputs[0].iter().all(|&x| x == 0.0));
    }
}
