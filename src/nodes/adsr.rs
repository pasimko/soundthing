use super::{PortInfo, Node, graph, NodeUi, PortDescriptions, SAMPLERATE, drain_messages};
use std::sync::mpsc::channel;
use std::sync::mpsc::Sender;
use std::sync::mpsc::Receiver;

pub enum AdsrMessage {
    Gate(bool),
    Attack(f32),
    Decay(f32),
    Sustain(f32),
    Release(f32),
}

#[derive(Debug, Clone)]
pub struct AdsrParameters {
    pub gate: bool,
    pub a: f32, // seconds to rise to full level
    pub d: f32, // seconds to fall from full level to the sustain level
    pub s: f32, // sustain level, 0 to 1
    pub r: f32, // seconds to fall from full level to silence
    pub sender: Sender<AdsrMessage>,
}

impl AdsrParameters {
    fn handle_message(&mut self, msg: AdsrMessage) {
        match msg {
            AdsrMessage::Gate(val) => self.gate = val,
            AdsrMessage::Attack(val) => self.a = val,
            AdsrMessage::Decay(val) => self.d = val,
            AdsrMessage::Sustain(val) => self.s = val,
            AdsrMessage::Release(val) => self.r = val,
        }
    }
}

enum State {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release
}

// Each segment is an exponential curve aimed a little past its destination, so it arrives
// there (rather than creeping up on it forever) in exactly the requested time. Larger values
// are closer to linear.
const CURVE: f32 = 0.1;

/// Per-sample multiplier that closes `1 / ratio` of the remaining distance to the aim point
/// in `seconds`.
fn pole(ratio: f32, seconds: f32) -> f32 {
    // `max` also turns a NaN time into a single sample
    ratio.powf(1.0 / (seconds * SAMPLERATE as f32).max(1.0))
}

impl NodeUi for AdsrParameters {
    fn duplicate(&self) -> Option<(Box<dyn Node>, Box<dyn NodeUi>)> {
        let (node, params) = AdsrNode::from_params(self.clone());
        Some((Box::new(node), Box::new(params)))
    }

    fn title(&self) -> String {
        "ADSR".to_string()
    }

    fn ports(&self) -> PortDescriptions {
        PortDescriptions::with_ports(
            vec![
                PortInfo::input("signal", "The signal the envelope shapes. If nothing is connected, the output is the envelope itself, a 0 to 1 control signal."),
                PortInfo::input("gate", "Stays above 0 while the note is held: it attacks, decays to the sustain level, then holds. At or below 0 it releases. Overrides the gate button."),
                PortInfo::input("attack time", "Seconds to rise to full level. Overrides the slider."),
                PortInfo::input("decay time", "Seconds to fall from full level to the sustain level. Overrides the slider."),
                PortInfo::input("sustain level", "Level held while the gate stays on, from 0 to 1. Overrides the slider."),
                PortInfo::input("release time", "Seconds to fall from full level to silence once the gate closes. Overrides the slider."),
            ],
            vec![
                PortInfo::output("signal out", "The input signal scaled by the envelope."),
            ]
        )
    }

    fn draw(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                let a_res = ui.add(egui::Slider::new(&mut self.a, 0.001..=10.0)
                    .text("attack")
                    .suffix(" s")
                    .logarithmic(true));
                let d_res = ui.add(egui::Slider::new(&mut self.d, 0.001..=10.0)
                    .text("decay")
                    .suffix(" s")
                    .logarithmic(true));
                let s_res = ui.add(egui::Slider::new(&mut self.s, 0.0..=1.0)
                    .text("sustain"));
                let r_res = ui.add(egui::Slider::new(&mut self.r, 0.001..=10.0)
                    .text("release")
                    .suffix(" s")
                    .logarithmic(true));
                let gate_res = ui.add(egui::Button::new("gate").selected(self.gate));
                if gate_res.clicked() {
                    self.gate = !self.gate;
                    let _ = self.sender.send(AdsrMessage::Gate(self.gate));
                }
                if a_res.changed() {
                    let _ = self.sender.send(AdsrMessage::Attack(self.a));
                }
                if d_res.changed() {
                    let _ = self.sender.send(AdsrMessage::Decay(self.d));
                }
                if s_res.changed() {
                    let _ = self.sender.send(AdsrMessage::Sustain(self.s));
                }
                if r_res.changed() {
                    let _ = self.sender.send(AdsrMessage::Release(self.r));
                }
            });
        });
    }
}

pub struct AdsrNode {
    params: AdsrParameters,
    state: State,
    gate_high: bool, // was the gate above 0 on the previous sample?
    level: f32, // the envelope's current output, 0 to 1
    msg_receiver: Receiver<AdsrMessage>,
}

impl AdsrNode {
    pub fn new() -> (Self, AdsrParameters) {
        let (msg_sender, msg_receiver) = channel();
        let params = AdsrParameters {
            gate: false,
            a: 0.1,
            d: 0.1,
            s: 0.8,
            r: 1.0,
            sender: msg_sender,
        };
        Self::build(params, msg_receiver)
    }

    pub fn from_params(mut params: AdsrParameters) -> (Self, AdsrParameters) {
        let (msg_sender, msg_receiver) = channel();
        params.sender = msg_sender;
        Self::build(params, msg_receiver)
    }

    fn build(params: AdsrParameters, msg_receiver: Receiver<AdsrMessage>) -> (Self, AdsrParameters) {
        let handler = params.clone();
        (Self {
            params,
            msg_receiver,
            state: State::Idle,
            gate_high: false,
            level: 0.,
        }, handler)
    }

    /// Advances the envelope by one sample and returns its level.
    /// Times are in seconds; `sustain` is expected to be within 0 to 1.
    fn step(&mut self, gate_high: bool, attack: f32, decay: f32, sustain: f32, release: f32) -> f32 {
        if gate_high && !self.gate_high {
            // Retriggering mid-release attacks from wherever the level is, so there's no click
            self.state = State::Attack;
        } else if !gate_high && matches!(self.state, State::Attack | State::Decay | State::Sustain) {
            self.state = State::Release;
        }
        self.gate_high = gate_high;

        match self.state {
            State::Idle => {},
            State::Attack => {
                let p = pole(CURVE / (1. + CURVE), attack);
                self.level = (1. - p) * (1. + CURVE) + p * self.level;
                if self.level >= 1. {
                    self.level = 1.;
                    self.state = State::Decay;
                }
            },
            State::Decay => {
                let p = pole(CURVE / (1. - sustain + CURVE), decay);
                self.level = (1. - p) * (sustain - CURVE) + p * self.level;
                if self.level <= sustain {
                    self.level = sustain;
                    self.state = State::Sustain;
                }
            },
            // Follows `sustain` so turning the knob (or modulating it) works while held
            State::Sustain => self.level = sustain,
            State::Release => {
                let p = pole(CURVE / (1. + CURVE), release);
                self.level = p * self.level - (1. - p) * CURVE;
                if self.level <= 0. {
                    self.level = 0.;
                    self.state = State::Idle;
                }
            },
        }
        self.level
    }
}

impl Node for AdsrNode {
    fn process(&mut self, inputs: &[(graph::PortId, &[f32])], outputs: &mut [Vec<f32>]) {
        let output = &mut outputs[0];
        drain_messages(&self.msg_receiver, |msg| self.params.handle_message(msg));
        // If we have inputs, use these buffers
        let mut signal_buf = None;
        let mut gate_buf = None;
        let mut attack_buf = None;
        let mut decay_buf = None;
        let mut sustain_buf = None;
        let mut release_buf = None;
        for (port, buffer) in inputs {
            // FRAGILE: These have to match the order the ports are declared in the UI
            // A possible future solution will have something like PortType rather than
            // portId, but I am concerned that I might want to have one node with duplicate
            // inputs in the future, so I'm not doing that yet
            match port.0 {
                0 => signal_buf = Some(buffer),
                1 => gate_buf = Some(buffer),
                2 => attack_buf = Some(buffer),
                3 => decay_buf = Some(buffer),
                4 => sustain_buf = Some(buffer),
                5 => release_buf = Some(buffer),
                _ => {}
            }
        }
        for i in 0..output.len() {
            let signal = signal_buf
                .map(|b| b[i])
                .unwrap_or(1.0);

            let gate = gate_buf
                .map(|b| b[i])
                .unwrap_or(if self.params.gate {1.} else {0.0});

            let attack = attack_buf
                .map(|b| b[i])
                .unwrap_or(self.params.a);

            let decay = decay_buf
                .map(|b| b[i])
                .unwrap_or(self.params.d);

            // `max` then `min` rather than `clamp`, so a NaN becomes 0 instead of propagating
            let sustain = sustain_buf
                .map(|b| b[i])
                .unwrap_or(self.params.s)
                .max(0.0)
                .min(1.0);

            let release = release_buf
                .map(|b| b[i])
                .unwrap_or(self.params.r);

            output[i] = signal * self.step(gate > 0.0, attack, decay, sustain, release);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(node: &mut AdsrNode, gate: bool, seconds: f32, a: f32, d: f32, s: f32, r: f32) -> Vec<f32> {
        (0..(seconds * SAMPLERATE as f32) as usize)
            .map(|_| node.step(gate, a, d, s, r))
            .collect()
    }

    #[test]
    fn silent_until_gated() {
        let (mut node, _) = AdsrNode::new();
        assert!(run(&mut node, false, 0.1, 0.1, 0.1, 0.5, 0.1).iter().all(|&x| x == 0.));
    }

    #[test]
    fn reaches_full_level_in_the_attack_time() {
        let (mut node, _) = AdsrNode::new();
        let out = run(&mut node, true, 0.2, 0.1, 0.1, 0.5, 0.1);
        let peak_at = out.iter().position(|&x| x >= 1.).unwrap();
        let expected = (0.1 * SAMPLERATE as f32) as usize;
        assert!(peak_at.abs_diff(expected) <= 2, "peaked at {peak_at}, expected {expected}");
    }

    #[test]
    fn decays_to_and_holds_the_sustain_level() {
        let (mut node, _) = AdsrNode::new();
        let out = run(&mut node, true, 0.5, 0.01, 0.1, 0.4, 0.1);
        assert_eq!(*out.last().unwrap(), 0.4);
    }

    #[test]
    fn releases_to_silence_in_the_release_time() {
        let (mut node, _) = AdsrNode::new();
        run(&mut node, true, 0.5, 0.01, 0.01, 1.0, 0.1);
        let out = run(&mut node, false, 0.2, 0.01, 0.01, 1.0, 0.1);
        let silent_at = out.iter().position(|&x| x == 0.).unwrap();
        let expected = (0.1 * SAMPLERATE as f32) as usize;
        assert!(silent_at.abs_diff(expected) <= 2, "silent at {silent_at}, expected {expected}");
        assert!(out[silent_at..].iter().all(|&x| x == 0.));
    }

    #[test]
    fn retrigger_during_release_resumes_from_current_level() {
        let (mut node, _) = AdsrNode::new();
        run(&mut node, true, 0.2, 0.01, 0.01, 1.0, 0.5);
        let released = run(&mut node, false, 0.05, 0.01, 0.01, 1.0, 0.5);
        let level = *released.last().unwrap();
        let next = node.step(true, 0.01, 0.01, 1.0, 0.5);
        assert!(level > 0. && (next - level).abs() < 0.05, "{level} -> {next}");
    }

    #[test]
    fn degenerate_settings_never_produce_nan() {
        for s in [0.0, 1.0] {
            for t in [0.0, -1.0, f32::NAN, 1e-9] {
                let (mut node, _) = AdsrNode::new();
                for gate in [true, false, true] {
                    for x in run(&mut node, gate, 0.05, t, t, s, t) {
                        assert!((0.0..=1.0).contains(&x), "s={s} t={t} gave {x}");
                    }
                }
            }
        }
    }
}
