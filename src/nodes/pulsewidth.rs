use std::sync::mpsc::Sender;
use crate::nodes::graph::PortId;
use super::{PortInfo, Node, SAMPLERATE, NodeUi, PortDescriptions, drain_messages};

use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;

pub enum PulsewidthMessage {
    Width(f32),
}

#[derive(Debug, Clone)]
pub struct PulsewidthParameters {
    pub width: f32, // seconds
    pub sender: Sender<PulsewidthMessage>,
}

impl PulsewidthParameters {
    fn handle_message(&mut self, msg: PulsewidthMessage) {
        match msg {
            PulsewidthMessage::Width(val) => self.width = val,
        }
    }
}

impl NodeUi for PulsewidthParameters {
    fn duplicate(&self) -> Option<(Box<dyn Node>, Box<dyn NodeUi>)> {
        let (node, params) = PulsewidthNode::from_params(self.clone());
        Some((Box::new(node), Box::new(params)))
    }

    fn title(&self) -> String {
        "pulsewidth".to_string()
    }

    fn ports(&self) -> PortDescriptions {
        PortDescriptions::with_ports(
            vec![
                PortInfo::input("pulse in", "Each time this goes from zero to nonzero, the output goes high for the pulse width. Triggering again restarts the timer."),
                PortInfo::input("width", "How long the output stays high, in seconds. Overrides the slider."),
            ],
            vec![
                PortInfo::output("pulse out", "1 while the pulse is being held, otherwise 0."),
            ]
        )
    }

    fn draw(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_width(100.0);
                let width_res = ui.add(egui::Slider::new(&mut self.width, 0.001..=10.0)
                    .text("width")
                    .suffix(" s")
                    .logarithmic(true));
                if width_res.changed() {
                    let _ = self.sender.send(PulsewidthMessage::Width(self.width));
                }
            });
        });
    }
}

pub struct PulsewidthNode {
    params: PulsewidthParameters,
    remaining: u64, // samples of output left to hold high
    trigger_high: bool, // was the input nonzero on the previous sample?
    msg_receiver: Receiver<PulsewidthMessage>,
}

impl PulsewidthNode {
    pub fn new() -> (Self, PulsewidthParameters) {
        let (msg_sender, msg_receiver) = channel();
        let params = PulsewidthParameters {
            width: 0.1,
            sender: msg_sender,
        };
        Self::build(params, msg_receiver)
    }

    pub fn from_params(mut params: PulsewidthParameters) -> (Self, PulsewidthParameters) {
        let (msg_sender, msg_receiver) = channel();
        params.sender = msg_sender;
        Self::build(params, msg_receiver)
    }

    fn build(params: PulsewidthParameters, msg_receiver: Receiver<PulsewidthMessage>) -> (Self, PulsewidthParameters) {
        let handler = params.clone();
        (Self {
            params,
            remaining: 0,
            trigger_high: false,
            msg_receiver,
        }, handler)
    }
}

impl Node for PulsewidthNode {
    fn process(&mut self, inputs: &[(PortId, &[f32])], outputs: &mut [Vec<f32>]) {
        let output = &mut outputs[0];
        drain_messages(&self.msg_receiver, |msg| self.params.handle_message(msg));
        // If we have inputs, use these buffers
        let mut pulse_buf = None;
        let mut width_buf = None;
        for (port, buffer) in inputs {
            // FRAGILE: These have to match the order the ports are declared in the UI
            match port.0 {
                0 => pulse_buf = Some(buffer),
                1 => width_buf = Some(buffer),
                _ => {}
            }
        }
        for i in 0..output.len() {
            let pulse = pulse_buf
                .map(|b| b[i])
                .unwrap_or(0.0);

            let width = width_buf
                .map(|b| b[i])
                .unwrap_or(self.params.width);

            let high = pulse.abs() > 0.0;
            if high && !self.trigger_high {
                // Always at least one sample, so even a zero, negative or NaN width still passes
                // the trigger along (`max` turns a NaN into 1)
                self.remaining = (width as f64 * SAMPLERATE as f64).round().max(1.0) as u64;
            }
            self.trigger_high = high;

            output[i] = if self.remaining > 0 {
                self.remaining -= 1;
                1.0
            } else {
                0.0
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(node: &mut PulsewidthNode, pulse: &[f32], width: Option<f32>) -> Vec<f32> {
        let mut outputs = [vec![0.; pulse.len()]];
        match width {
            Some(w) => {
                let width = vec![w; pulse.len()];
                node.process(&[(PortId(0), pulse), (PortId(1), &width)], &mut outputs);
            }
            None => node.process(&[(PortId(0), pulse)], &mut outputs),
        }
        let [out] = outputs;
        out
    }

    fn one_pulse_then_silence(total: usize) -> Vec<f32> {
        let mut v = vec![0.0; total];
        v[10] = 1.0;
        v
    }

    #[test]
    fn holds_for_the_width_in_seconds() {
        let (mut node, _) = PulsewidthNode::new(); // 0.1 s
        let out = run(&mut node, &one_pulse_then_silence(10_000), None);
        let high: Vec<usize> = out.iter().enumerate().filter(|(_, &x)| x == 1.0).map(|(i, _)| i).collect();
        assert_eq!(high.len(), 4800);
        assert_eq!((high[0], *high.last().unwrap()), (10, 10 + 4799));
        assert!(out.iter().all(|&x| x == 0.0 || x == 1.0));
    }

    #[test]
    fn a_width_port_overrides_the_slider() {
        let (mut node, _) = PulsewidthNode::new();
        let out = run(&mut node, &one_pulse_then_silence(2_000), Some(0.01));
        assert_eq!(out.iter().filter(|&&x| x == 1.0).count(), 480);
    }

    #[test]
    fn retriggering_restarts_the_timer() {
        let (mut node, _) = PulsewidthNode::new();
        let mut pulse = vec![0.0; 20_000];
        pulse[0] = 1.0;
        pulse[3_000] = 1.0; // before the first 4800 samples are up
        let out = run(&mut node, &pulse, None);
        assert_eq!(out.iter().filter(|&&x| x == 1.0).count(), 3_000 + 4_800);
        assert_eq!(out[7_799], 1.0);
        assert_eq!(out[7_800], 0.0);
    }

    #[test]
    fn a_held_input_is_one_trigger() {
        let (mut node, _) = PulsewidthNode::new();
        let mut pulse = vec![1.0; 10_000]; // held long past the width
        pulse.extend(vec![0.0; 10_000]);
        let out = run(&mut node, &pulse, None);
        assert_eq!(out.iter().filter(|&&x| x == 1.0).count(), 4_800);
        assert_eq!(out[4_800], 0.0);
    }

    #[test]
    fn the_hold_carries_across_blocks() {
        let (mut node, _) = PulsewidthNode::new();
        let mut high = 0;
        high += run(&mut node, &[1.0, 0.0, 0.0, 0.0], None).iter().filter(|&&x| x == 1.0).count();
        for _ in 0..100 {
            high += run(&mut node, &[0.0; 128], None).iter().filter(|&&x| x == 1.0).count();
        }
        assert_eq!(high, 4_800);
    }

    #[test]
    fn degenerate_widths_still_pass_the_trigger_along() {
        for w in [0.0, -1.0, f32::NAN] {
            let (mut node, _) = PulsewidthNode::new();
            let out = run(&mut node, &one_pulse_then_silence(100), Some(w));
            assert_eq!(out.iter().filter(|&&x| x == 1.0).count(), 1, "width {w}");
        }
    }
}
