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

// What each segment looks like, as a function of how far through it we are (0 to 1). These are
// the closed forms of what `AdsrNode::step` does sample by sample, and are what the graph draws.
fn attack_level(progress: f32) -> f32 {
    (1. + CURVE) * (1. - (CURVE / (1. + CURVE)).powf(progress))
}

fn decay_level(progress: f32, sustain: f32) -> f32 {
    let aim = sustain - CURVE;
    aim + (1. - aim) * (CURVE / (1. - sustain + CURVE)).powf(progress)
}

fn release_level(progress: f32, from: f32) -> f32 {
    -CURVE + (from + CURVE) * (CURVE / (from + CURVE)).powf(progress)
}

// The graph gives each time a lane to move in, on the same log scale as the old sliders, so
// a 2 ms attack and a 10 s release are both easy to grab.
const GRAPH_SIZE: egui::Vec2 = egui::vec2(220.0, 110.0);
const GRAPH_MARGIN: f32 = 9.0;
const TIME_MIN: f32 = 0.001;
const TIME_MAX: f32 = 10.0;
// Width of the lanes: attack, decay, release, and half a lane of sustain between them
const LANES: f32 = 3.5;
const CURVE_SAMPLES: usize = 24;

fn time_to_lane(seconds: f32) -> f32 {
    ((seconds.max(TIME_MIN) / TIME_MIN).ln() / (TIME_MAX / TIME_MIN).ln()).clamp(0.0, 1.0)
}

fn lane_to_time(lane: f32) -> f32 {
    TIME_MIN * (TIME_MAX / TIME_MIN).powf(lane.clamp(0.0, 1.0))
}

fn level_to_y(inner: egui::Rect, level: f32) -> f32 {
    inner.bottom() - level.clamp(0.0, 1.0) * inner.height()
}

/// Where the envelope's corners sit in the graph.
struct GraphPoints {
    start: egui::Pos2,
    attack: egui::Pos2,
    decay: egui::Pos2,
    release_start: egui::Pos2,
    release: egui::Pos2,
}

impl AdsrParameters {
    fn graph_points(&self, inner: egui::Rect) -> GraphPoints {
        let lane = inner.width() / LANES;
        let start = egui::pos2(inner.left(), level_to_y(inner, 0.));
        let attack = egui::pos2(start.x + time_to_lane(self.a) * lane, level_to_y(inner, 1.));
        let decay = egui::pos2(attack.x + time_to_lane(self.d) * lane, level_to_y(inner, self.s));
        let release_start = egui::pos2(decay.x + 0.5 * lane, decay.y);
        let release = egui::pos2(release_start.x + time_to_lane(self.r) * lane, level_to_y(inner, 0.));
        GraphPoints { start, attack, decay, release_start, release }
    }
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
                PortInfo::input("attack time", "Seconds to rise to full level. Overrides the graph."),
                PortInfo::input("decay time", "Seconds to fall from full level to the sustain level. Overrides the graph."),
                PortInfo::input("sustain level", "Level held while the gate stays on, from 0 to 1. Overrides the graph."),
                PortInfo::input("release time", "Seconds to fade to silence once the gate closes, from wherever the level was. Overrides the graph."),
            ],
            vec![
                PortInfo::output("signal out", "The input signal scaled by the envelope."),
            ]
        )
    }

    fn draw(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            let (frame, painter) = ui.allocate_painter(GRAPH_SIZE, egui::Sense::hover());
            let inner = frame.rect.shrink(GRAPH_MARGIN);
            let lane = inner.width() / LANES;

            // Drag a corner to set its time; the decay corner also sets the sustain level
            let points = self.graph_points(inner);
            let handle = |ui: &egui::Ui, name: &str, at: egui::Pos2| ui.interact(
                egui::Rect::from_center_size(at, egui::Vec2::splat(18.0)),
                ui.id().with(name),
                egui::Sense::drag(),
            );
            let attack_handle = handle(ui, "attack", points.attack);
            let decay_handle = handle(ui, "decay", points.decay);
            let release_handle = handle(ui, "release", points.release);

            let mut changed = [false; 4];
            if let Some(pointer) = attack_handle.interact_pointer_pos().filter(|_| attack_handle.dragged()) {
                self.a = lane_to_time((pointer.x - points.start.x) / lane);
                changed[0] = true;
            }
            if let Some(pointer) = decay_handle.interact_pointer_pos().filter(|_| decay_handle.dragged()) {
                self.d = lane_to_time((pointer.x - points.attack.x) / lane);
                self.s = ((inner.bottom() - pointer.y) / inner.height()).clamp(0.0, 1.0);
                changed[1] = true;
                changed[2] = true;
            }
            if let Some(pointer) = release_handle.interact_pointer_pos().filter(|_| release_handle.dragged()) {
                self.r = lane_to_time((pointer.x - points.release_start.x) / lane);
                changed[3] = true;
            }

            // Draw from the updated values, so the corner doesn't lag the pointer by a frame
            let points = self.graph_points(inner);
            let visuals = ui.visuals();
            painter.rect(
                frame.rect,
                2.0,
                visuals.extreme_bg_color,
                visuals.widgets.noninteractive.bg_stroke,
                egui::StrokeKind::Inside,
            );
            let segment = |from: egui::Pos2, to: egui::Pos2, level: &dyn Fn(f32) -> f32| {
                (0..=CURVE_SAMPLES).map(move |k| {
                    let progress = k as f32 / CURVE_SAMPLES as f32;
                    egui::pos2(
                        from.x + (to.x - from.x) * progress,
                        level_to_y(inner, level(progress)),
                    )
                }).collect::<Vec<_>>()
            };
            let mut curve = segment(points.start, points.attack, &attack_level);
            curve.extend(segment(points.attack, points.decay, &|p| decay_level(p, self.s)));
            curve.push(points.release_start);
            curve.extend(segment(points.release_start, points.release, &|p| release_level(p, self.s)));
            painter.add(egui::Shape::line(curve, egui::Stroke::new(2.0_f32, visuals.text_color())));
            for (corner, response) in [
                (points.attack, &attack_handle),
                (points.decay, &decay_handle),
                (points.release, &release_handle),
            ] {
                let radius = if response.hovered() || response.dragged() { 7.0 } else { 5.0 };
                painter.circle(
                    corner,
                    radius,
                    visuals.selection.bg_fill,
                    egui::Stroke::new(1.5_f32, visuals.text_color()),
                );
            }

            // The same four values, for typing in exact ones
            ui.horizontal_wrapped(|ui| {
                ui.set_max_width(GRAPH_SIZE.x);
                let a_res = ui.add(egui::DragValue::new(&mut self.a)
                    .range(TIME_MIN..=TIME_MAX).speed(0.005).prefix("A ").suffix(" s"));
                let d_res = ui.add(egui::DragValue::new(&mut self.d)
                    .range(TIME_MIN..=TIME_MAX).speed(0.005).prefix("D ").suffix(" s"));
                let s_res = ui.add(egui::DragValue::new(&mut self.s)
                    .range(0.0..=1.0).speed(0.005).prefix("S "));
                let r_res = ui.add(egui::DragValue::new(&mut self.r)
                    .range(TIME_MIN..=TIME_MAX).speed(0.005).prefix("R ").suffix(" s"));
                changed[0] |= a_res.changed();
                changed[1] |= d_res.changed();
                changed[2] |= s_res.changed();
                changed[3] |= r_res.changed();
            });

            let gate_res = ui.add(egui::Button::new("gate").selected(self.gate));
            if gate_res.clicked() {
                self.gate = !self.gate;
                let _ = self.sender.send(AdsrMessage::Gate(self.gate));
            }
            if changed[0] {
                let _ = self.sender.send(AdsrMessage::Attack(self.a));
            }
            if changed[1] {
                let _ = self.sender.send(AdsrMessage::Decay(self.d));
            }
            if changed[2] {
                let _ = self.sender.send(AdsrMessage::Sustain(self.s));
            }
            if changed[3] {
                let _ = self.sender.send(AdsrMessage::Release(self.r));
            }
        });
    }
}

pub struct AdsrNode {
    params: AdsrParameters,
    state: State,
    gate_high: bool, // was the gate above 0 on the previous sample?
    level: f32, // the envelope's current output, 0 to 1
    release_from: f32, // the level the gate closed at, which the release fades from
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
            release_from: 0.,
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
            self.release_from = self.level;
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
            // Fades to silence in `release` seconds from wherever the gate closed, so a short note
            // doesn't get a shorter release than a long one
            State::Release => {
                let p = pole(CURVE / (self.release_from + CURVE), release);
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

    fn silent_at(out: &[f32]) -> usize {
        out.iter().position(|&x| x == 0.).unwrap()
    }

    #[test]
    fn releases_in_the_release_time_from_any_level() {
        let expected = (0.1 * SAMPLERATE as f32) as usize;

        // from a sustain level below full
        let (mut node, _) = AdsrNode::new();
        run(&mut node, true, 0.5, 0.01, 0.01, 0.3, 0.1);
        let out = run(&mut node, false, 0.2, 0.01, 0.01, 0.3, 0.1);
        assert!(silent_at(&out).abs_diff(expected) <= 2, "from sustain: {}", silent_at(&out));

        // from part of the way up a long attack
        let (mut node, _) = AdsrNode::new();
        run(&mut node, true, 0.05, 0.5, 0.5, 0.3, 0.1);
        assert!(node.level > 0.1 && node.level < 0.9);
        let out = run(&mut node, false, 0.2, 0.5, 0.5, 0.3, 0.1);
        assert!(silent_at(&out).abs_diff(expected) <= 2, "from mid-attack: {}", silent_at(&out));
    }

    #[test]
    fn the_graph_curves_match_what_the_node_does() {
        let close = |a: f32, b: f32, what: &str| assert!((a - b).abs() < 1e-3, "{what}: {a} vs {b}");
        let n = (0.1 * SAMPLERATE as f32) as usize;
        let (mut node, _) = AdsrNode::new();

        for i in 1..n {
            let level = node.step(true, 0.1, 0.1, 0.4, 0.1);
            if i % 400 == 0 {
                close(level, attack_level(i as f32 / n as f32), "attack");
            }
        }
        while !matches!(node.state, State::Decay) {
            node.step(true, 0.1, 0.1, 0.4, 0.1);
        }
        for j in 1..n {
            let level = node.step(true, 0.1, 0.1, 0.4, 0.1);
            if j % 400 == 0 {
                close(level, decay_level(j as f32 / n as f32, 0.4), "decay");
            }
        }
        while !matches!(node.state, State::Sustain) {
            node.step(true, 0.1, 0.1, 0.4, 0.1);
        }
        for j in 1..n {
            let level = node.step(false, 0.1, 0.1, 0.4, 0.1);
            if j % 400 == 0 {
                close(level, release_level(j as f32 / n as f32, 0.4), "release");
            }
        }
    }

    #[test]
    fn the_curves_start_and_end_where_the_corners_are() {
        for s in [0.0, 0.3, 1.0] {
            assert!((attack_level(0.0)).abs() < 1e-6 && (attack_level(1.0) - 1.0).abs() < 1e-6);
            assert!((decay_level(0.0, s) - 1.0).abs() < 1e-6 && (decay_level(1.0, s) - s).abs() < 1e-6);
            assert!((release_level(0.0, s) - s).abs() < 1e-6 && release_level(1.0, s).abs() < 1e-6);
        }
    }

    #[test]
    fn lanes_round_trip_times_and_clamp() {
        for t in [0.001, 0.01, 0.1, 1.0, 10.0] {
            assert!((lane_to_time(time_to_lane(t)) - t).abs() <= t * 1e-4, "{t}");
        }
        assert_eq!(time_to_lane(0.0), 0.0);
        assert_eq!(time_to_lane(f32::NAN), 0.0);
        assert_eq!(time_to_lane(1000.0), 1.0);
        assert_eq!(lane_to_time(-1.0), TIME_MIN);
        assert_eq!(lane_to_time(2.0), TIME_MAX);
    }

    #[test]
    fn the_graph_corners_always_fit_in_the_frame() {
        let inner = egui::Rect::from_min_size(egui::pos2(9., 9.), egui::vec2(202., 92.));
        for t in [0.0, 0.001, 1.0, 10.0, 99.0, f32::NAN] {
            for s in [0.0, 0.5, 1.0, 5.0, -1.0] {
                let (_, mut params) = AdsrNode::new();
                (params.a, params.d, params.s, params.r) = (t, t, s, t);
                let p = params.graph_points(inner);
                for corner in [p.start, p.attack, p.decay, p.release_start, p.release] {
                    assert!(inner.expand(0.01).contains(corner), "t {t} s {s}: {corner:?}");
                }
            }
        }
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
