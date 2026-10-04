use std::f32::consts::PI;
use std::sync::mpsc::Sender;
use crate::nodes::graph::PortId;
use super::{PortInfo, Node, SAMPLERATE, NodeUi, PortDescriptions, drain_messages};

use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;

// How sharply the exponential curve bends. Higher is a faster start and a longer tail.
const EXPONENTIAL_BEND: f32 = 5.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InterpMode {
    /// Constant speed.
    Linear,
    /// Eases in and out along half a cosine wave.
    Cosine,
    /// Eases in and out along a cubic (3t² - 2t³). Slightly flatter at the ends than cosine.
    Smoothstep,
    /// Starts fast and settles slowly, like an analog glide.
    Exponential,
    /// Constant ratio per unit time, so pitches slide evenly. Needs positive values; anything
    /// else falls back to linear.
    Geometric,
}

impl InterpMode {
    pub const ALL: [InterpMode; 5] = [
        InterpMode::Linear,
        InterpMode::Cosine,
        InterpMode::Smoothstep,
        InterpMode::Exponential,
        InterpMode::Geometric,
    ];

    pub fn name(self) -> &'static str {
        match self {
            InterpMode::Linear => "linear",
            InterpMode::Cosine => "cosine",
            InterpMode::Smoothstep => "smoothstep",
            InterpMode::Exponential => "exponential",
            InterpMode::Geometric => "geometric",
        }
    }

    /// The value `t` (0 to 1) of the way from `from` to `to`.
    pub fn interpolate(self, from: f32, to: f32, t: f32) -> f32 {
        let lerp = |t: f32| from + (to - from) * t;
        match self {
            InterpMode::Linear => lerp(t),
            InterpMode::Cosine => lerp((1. - (PI * t).cos()) / 2.),
            InterpMode::Smoothstep => lerp(t * t * (3. - 2. * t)),
            InterpMode::Exponential => {
                lerp((1. - (-EXPONENTIAL_BEND * t).exp()) / (1. - (-EXPONENTIAL_BEND).exp()))
            },
            InterpMode::Geometric => {
                if from > 0. && to > 0. {
                    from * (to / from).powf(t)
                } else {
                    lerp(t)
                }
            },
        }
    }
}

pub enum InterpolatorMessage {
    Time(f32),
    Mode(InterpMode),
}

#[derive(Debug, Clone)]
pub struct InterpolatorParameters {
    pub time: f32, // seconds each glide takes
    pub mode: InterpMode,
    pub sender: Sender<InterpolatorMessage>,
}

impl InterpolatorParameters {
    fn handle_message(&mut self, msg: InterpolatorMessage) {
        match msg {
            InterpolatorMessage::Time(val) => self.time = val,
            InterpolatorMessage::Mode(val) => self.mode = val,
        }
    }
}

impl NodeUi for InterpolatorParameters {
    fn duplicate(&self) -> Option<(Box<dyn Node>, Box<dyn NodeUi>)> {
        let (node, params) = InterpolatorNode::from_params(self.clone());
        Some((Box::new(node), Box::new(params)))
    }

    fn title(&self) -> String {
        "Interpolator".to_string()
    }

    fn ports(&self) -> PortDescriptions {
        PortDescriptions::with_ports(
            vec![
                PortInfo::input("signal in", "The value to follow. Each time it changes, the output glides to the new value."),
                PortInfo::input("time", "How long each glide takes, in seconds. Overrides the slider."),
            ],
            vec![
                PortInfo::output("signal out", "The input, with its jumps smoothed into glides."),
            ]
        )
    }

    fn draw(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_width(100.0);
                let time_res = ui.add(egui::Slider::new(&mut self.time, 0.001..=10.0)
                    .text("time")
                    .suffix(" s")
                    .logarithmic(true));
                if time_res.changed() {
                    let _ = self.sender.send(InterpolatorMessage::Time(self.time));
                }
                egui::ComboBox::from_label("curve")
                    .selected_text(self.mode.name())
                    .show_ui(ui, |ui| {
                        for mode in InterpMode::ALL {
                            if ui.selectable_value(&mut self.mode, mode, mode.name()).changed() {
                                let _ = self.sender.send(InterpolatorMessage::Mode(self.mode));
                            }
                        }
                    });
            });
        });
    }
}

pub struct InterpolatorNode {
    params: InterpolatorParameters,
    started: bool, // has the first value arrived? The output starts there instead of gliding from 0
    from: f32, // where the current glide began
    to: f32, // the value the current glide is heading for
    progress: f64, // how far through the glide we are, 0 to 1
    level: f32, // the output
    msg_receiver: Receiver<InterpolatorMessage>,
}

impl InterpolatorNode {
    pub fn new() -> (Self, InterpolatorParameters) {
        let (msg_sender, msg_receiver) = channel();
        let params = InterpolatorParameters {
            time: 0.1,
            mode: InterpMode::Smoothstep,
            sender: msg_sender,
        };
        Self::build(params, msg_receiver)
    }

    pub fn from_params(mut params: InterpolatorParameters) -> (Self, InterpolatorParameters) {
        let (msg_sender, msg_receiver) = channel();
        params.sender = msg_sender;
        Self::build(params, msg_receiver)
    }

    fn build(params: InterpolatorParameters, msg_receiver: Receiver<InterpolatorMessage>) -> (Self, InterpolatorParameters) {
        let handler = params.clone();
        (Self {
            params,
            started: false,
            from: 0.,
            to: 0.,
            progress: 1.,
            level: 0.,
            msg_receiver,
        }, handler)
    }

    /// Advances one sample towards `target`, taking `time` seconds to get there.
    fn step(&mut self, target: f32, time: f32) -> f32 {
        // Ignore NaN and infinities rather than let them into the output for good. Until the
        // first real value arrives there's nothing to hold, so the output stays at 0.
        let target = if target.is_finite() {
            target
        } else if self.started {
            self.to
        } else {
            return self.level;
        };

        if !self.started {
            self.started = true;
            self.from = target;
            self.to = target;
            self.level = target;
            self.progress = 1.;
        } else if target != self.to {
            // Starting from the current level, not the old target, so a change mid-glide
            // doesn't jump
            self.from = self.level;
            self.to = target;
            self.progress = 0.;
        }

        if self.progress < 1. {
            // At least one sample, so a zero, negative or NaN time just jumps (`max` turns a
            // NaN into 1)
            let samples = (time as f64 * SAMPLERATE as f64).max(1.);
            self.progress = (self.progress + 1. / samples).min(1.);
            self.level = if self.progress >= 1. {
                self.to
            } else {
                self.params.mode.interpolate(self.from, self.to, self.progress as f32)
            };
        }
        self.level
    }
}

impl Node for InterpolatorNode {
    fn process(&mut self, inputs: &[(PortId, &[f32])], outputs: &mut [Vec<f32>]) {
        let output = &mut outputs[0];
        drain_messages(&self.msg_receiver, |msg| self.params.handle_message(msg));
        // If we have inputs, use these buffers
        let mut signal_buf = None;
        let mut time_buf = None;
        for (port, buffer) in inputs {
            // FRAGILE: These have to match the order the ports are declared in the UI
            match port.0 {
                0 => signal_buf = Some(buffer),
                1 => time_buf = Some(buffer),
                _ => {}
            }
        }
        for i in 0..output.len() {
            let signal = signal_buf
                .map(|b| b[i])
                .unwrap_or(0.0);

            let time = time_buf
                .map(|b| b[i])
                .unwrap_or(self.params.time);

            output[i] = self.step(signal, time);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(mode: InterpMode, time: f32) -> InterpolatorNode {
        let (mut node, _) = InterpolatorNode::new();
        node.params.mode = mode;
        node.params.time = time;
        node
    }

    fn run(node: &mut InterpolatorNode, signal: &[f32]) -> Vec<f32> {
        let mut outputs = [vec![0.; signal.len()]];
        node.process(&[(PortId(0), signal)], &mut outputs);
        let [out] = outputs;
        out
    }

    /// 100 samples at `before`, then `after` for `after_len` samples
    fn step_input(before: f32, after: f32, after_len: usize) -> Vec<f32> {
        let mut v = vec![before; 100];
        v.extend(vec![after; after_len]);
        v
    }

    #[test]
    fn every_curve_starts_and_ends_exactly_and_never_goes_backwards() {
        for mode in InterpMode::ALL {
            let mut n = node(mode, 0.01); // 480 samples
            let out = run(&mut n, &step_input(100., 400., 1000));
            assert_eq!(out[99], 100., "{mode:?}");
            assert!(out[100] >= 100. && out[100] < 120., "{mode:?} began with {}", out[100]);
            assert_eq!(out[100 + 479], 400., "{mode:?} should arrive after exactly 480 samples");
            assert!(out[100 + 478] < 400., "{mode:?} arrived early");
            assert!(out[99..].windows(2).all(|w| w[1] >= w[0]), "{mode:?} went backwards");
            // and the same going down
            let mut n = node(mode, 0.01);
            let out = run(&mut n, &step_input(400., 100., 1000));
            assert!(out[99..].windows(2).all(|w| w[1] <= w[0]), "{mode:?} went backwards on the way down");
            assert_eq!(*out.last().unwrap(), 100.);
        }
    }

    #[test]
    fn midpoints_match_each_curve() {
        let mid = |mode: InterpMode, from: f32, to: f32| mode.interpolate(from, to, 0.5);
        assert!((mid(InterpMode::Linear, 100., 400.) - 250.).abs() < 1e-3);
        assert!((mid(InterpMode::Cosine, 100., 400.) - 250.).abs() < 1e-3);
        assert!((mid(InterpMode::Smoothstep, 100., 400.) - 250.).abs() < 1e-3);
        // geometric: the midpoint is the geometric mean, so an octave glide passes through
        // the tritone: 220 * sqrt(2)
        assert!((mid(InterpMode::Geometric, 220., 440.) - 220. * 2f32.sqrt()).abs() < 1e-3);
        // exponential covers more than half the distance in the first half of the time
        assert!(mid(InterpMode::Exponential, 0., 1.) > 0.9);
    }

    #[test]
    fn geometric_falls_back_to_linear_for_non_positive_values() {
        for (from, to) in [(0., 100.), (-50., 50.), (100., 0.), (100., -100.)] {
            assert_eq!(
                InterpMode::Geometric.interpolate(from, to, 0.25),
                InterpMode::Linear.interpolate(from, to, 0.25),
            );
        }
    }

    #[test]
    fn the_first_value_is_not_a_glide_from_zero() {
        let mut n = node(InterpMode::Linear, 1.0);
        let out = run(&mut n, &vec![440.; 10]);
        assert!(out.iter().all(|&x| x == 440.));
    }

    #[test]
    fn changing_target_mid_glide_continues_from_where_it_is() {
        let mut n = node(InterpMode::Linear, 0.01);
        let mut input = step_input(0., 100., 240); // halfway through the glide...
        input.extend(vec![0.; 480]); // ...then sent back down
        let out = run(&mut n, &input);
        let at_turn = out[100 + 239];
        assert!(at_turn > 40. && at_turn < 60.);
        // no jump: the next sample is within one sample's travel of the last
        assert!((out[100 + 240] - at_turn).abs() < 1.0, "{} -> {}", at_turn, out[100 + 240]);
        assert_eq!(*out.last().unwrap(), 0.);
    }

    #[test]
    fn the_time_port_overrides_the_slider() {
        let mut n = node(InterpMode::Linear, 10.0);
        let signal = step_input(0., 100., 1000);
        let time = vec![0.005; signal.len()]; // 240 samples
        let mut outputs = [vec![0.; signal.len()]];
        n.process(&[(PortId(0), &signal), (PortId(1), &time)], &mut outputs);
        assert_eq!(outputs[0][100 + 239], 100.);
        assert!(outputs[0][100 + 238] < 100.);
    }

    #[test]
    fn zero_negative_and_nan_times_just_jump() {
        for t in [0.0, -3.0, f32::NAN] {
            let mut n = node(InterpMode::Cosine, t);
            let out = run(&mut n, &step_input(5., 9., 5));
            assert_eq!(out[100], 9., "time {t}");
        }
    }

    #[test]
    fn nan_and_infinite_inputs_hold_the_last_value() {
        let mut n = node(InterpMode::Linear, 0.01);
        let mut input = vec![3.; 50];
        input.extend([f32::NAN, f32::INFINITY, f32::NEG_INFINITY]);
        input.extend(vec![3.; 50]);
        let out = run(&mut n, &input);
        assert!(out.iter().all(|&x| x == 3.));
        // a NaN before any real value doesn't start the node
        let mut n = node(InterpMode::Linear, 0.01);
        let out = run(&mut n, &[f32::NAN, f32::NAN, 7.]);
        assert!(out.iter().all(|x| x.is_finite()));
        assert_eq!(out[2], 7.);
    }

    #[test]
    fn block_size_does_not_matter() {
        let signal = step_input(100., 400., 2000);
        let mut whole = node(InterpMode::Exponential, 0.02);
        let expected = run(&mut whole, &signal);
        let mut chunked = node(InterpMode::Exponential, 0.02);
        let got: Vec<f32> = signal.chunks(128).flat_map(|c| run(&mut chunked, c)).collect();
        assert_eq!(got, expected);
    }
}
