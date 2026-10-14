use biquad::{Biquad, Coefficients, DirectForm2Transposed, Type};
use std::sync::mpsc::Sender;
use crate::nodes::graph::PortId;
use super::{PortInfo, Node, SAMPLERATE, NodeUi, PortDescriptions, drain_messages};

use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;

// Keep the cutoff safely inside (0, Nyquist), where the coefficients are well behaved
const MIN_CUTOFF: f32 = 10.0;
const MAX_CUTOFF: f32 = SAMPLERATE as f32 * 0.49;
const MIN_RESONANCE: f32 = 0.1;
const MAX_RESONANCE: f32 = 40.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FilterMode {
    /// Passes what's below the cutoff.
    LowPass,
    /// Passes what's above the cutoff.
    HighPass,
    /// Passes a band around the cutoff.
    BandPass,
    /// Removes a band around the cutoff.
    Notch,
}

impl FilterMode {
    pub const ALL: [FilterMode; 4] = [
        FilterMode::LowPass,
        FilterMode::HighPass,
        FilterMode::BandPass,
        FilterMode::Notch,
    ];

    pub fn name(self) -> &'static str {
        match self {
            FilterMode::LowPass => "low pass",
            FilterMode::HighPass => "high pass",
            FilterMode::BandPass => "band pass",
            FilterMode::Notch => "notch",
        }
    }

    fn biquad_type(self) -> Type<f32> {
        match self {
            FilterMode::LowPass => Type::LowPass,
            FilterMode::HighPass => Type::HighPass,
            FilterMode::BandPass => Type::BandPass,
            FilterMode::Notch => Type::Notch,
        }
    }
}

/// Cutoff in Hz and resonance (Q), forced into the range the filter can handle. `max` then
/// `min` rather than `clamp`, so a NaN lands on the lower bound instead of propagating.
fn clamp_settings(cutoff: f32, resonance: f32) -> (f32, f32) {
    (
        cutoff.max(MIN_CUTOFF).min(MAX_CUTOFF),
        resonance.max(MIN_RESONANCE).min(MAX_RESONANCE),
    )
}

fn coefficients(mode: FilterMode, cutoff: f32, resonance: f32) -> Coefficients<f32> {
    let (cutoff, resonance) = clamp_settings(cutoff, resonance);
    let normalized_cutoff = cutoff / (SAMPLERATE as f32 / 2.0);
    // Can't fail with the clamped settings; if it ever did, letting the signal through
    // beats panicking in the audio thread
    Coefficients::from_normalized_params(mode.biquad_type(), normalized_cutoff, resonance)
        .unwrap_or(Coefficients { a1: 0., a2: 0., b0: 1., b1: 0., b2: 0. })
}

pub enum FilterMessage {
    Cutoff(f32),
    Resonance(f32),
    Mode(FilterMode),
}

#[derive(Debug, Clone)]
pub struct FilterParameters {
    pub cutoff: f32, // Hz
    pub resonance: f32, // Q
    pub mode: FilterMode,
    pub sender: Sender<FilterMessage>,
}

impl FilterParameters {
    fn handle_message(&mut self, msg: FilterMessage) {
        match msg {
            FilterMessage::Cutoff(val) => self.cutoff = val,
            FilterMessage::Resonance(val) => self.resonance = val,
            FilterMessage::Mode(val) => self.mode = val,
        }
    }
}

impl NodeUi for FilterParameters {
    fn duplicate(&self) -> Option<(Box<dyn Node>, Box<dyn NodeUi>)> {
        let (node, params) = FilterNode::from_params(self.clone());
        Some((Box::new(node), Box::new(params)))
    }

    fn title(&self) -> String {
        "Filter".to_string()
    }

    fn ports(&self) -> PortDescriptions {
        PortDescriptions::with_ports(
            vec![
                PortInfo::input("signal in", "The signal to filter."),
                PortInfo::input("cutoff", "Cutoff (or centre) frequency in Hz. Overrides the slider."),
                PortInfo::input("resonance", "How sharply the filter peaks (or, for band pass and notch, how narrow the band is). Overrides the slider."),
            ],
            vec![
                PortInfo::output("signal out", "The filtered signal."),
            ]
        )
    }

    fn draw(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_width(100.0);
                let cutoff_res = ui.add(egui::Slider::new(&mut self.cutoff, MIN_CUTOFF..=20_000.0)
                    .text("cutoff")
                    .suffix(" Hz")
                    .logarithmic(true));
                let resonance_res = ui.add(egui::Slider::new(&mut self.resonance, MIN_RESONANCE..=20.0)
                    .text("resonance")
                    .logarithmic(true));
                if cutoff_res.changed() {
                    let _ = self.sender.send(FilterMessage::Cutoff(self.cutoff));
                }
                if resonance_res.changed() {
                    let _ = self.sender.send(FilterMessage::Resonance(self.resonance));
                }
                egui::ComboBox::from_label("type")
                    .selected_text(self.mode.name())
                    .show_ui(ui, |ui| {
                        for mode in FilterMode::ALL {
                            if ui.selectable_value(&mut self.mode, mode, mode.name()).changed() {
                                let _ = self.sender.send(FilterMessage::Mode(self.mode));
                            }
                        }
                    });
            });
        });
    }
}

pub struct FilterNode {
    params: FilterParameters,
    filter: DirectForm2Transposed<f32>,
    // what the filter's coefficients were last built for, so they're only recomputed on a change
    applied: (FilterMode, f32, f32),
    msg_receiver: Receiver<FilterMessage>,
}

impl FilterNode {
    pub fn new() -> (Self, FilterParameters) {
        let (msg_sender, msg_receiver) = channel();
        let params = FilterParameters {
            cutoff: 1000.0,
            resonance: 0.707, // no peak: the flattest response
            mode: FilterMode::LowPass,
            sender: msg_sender,
        };
        Self::build(params, msg_receiver)
    }

    pub fn from_params(mut params: FilterParameters) -> (Self, FilterParameters) {
        let (msg_sender, msg_receiver) = channel();
        params.sender = msg_sender;
        Self::build(params, msg_receiver)
    }

    fn build(params: FilterParameters, msg_receiver: Receiver<FilterMessage>) -> (Self, FilterParameters) {
        let handler = params.clone();
        let (cutoff, resonance) = clamp_settings(params.cutoff, params.resonance);
        (Self {
            filter: DirectForm2Transposed::<f32>::new(coefficients(params.mode, cutoff, resonance)),
            applied: (params.mode, cutoff, resonance),
            params,
            msg_receiver,
        }, handler)
    }

    /// Filters one sample with the given settings.
    fn step(&mut self, input: f32, mode: FilterMode, cutoff: f32, resonance: f32) -> f32 {
        let (cutoff, resonance) = clamp_settings(cutoff, resonance);
        if (mode, cutoff, resonance) != self.applied {
            self.filter.update_coefficients(coefficients(mode, cutoff, resonance));
            self.applied = (mode, cutoff, resonance);
        }

        // A NaN or infinity going in would stay in the filter's memory and silence it for good
        let input = if input.is_finite() { input } else { 0.0 };
        let output = self.filter.run(input);
        if output.is_finite() {
            output
        } else {
            self.filter.reset_state();
            0.0
        }
    }
}

impl Node for FilterNode {
    fn process(&mut self, inputs: &[(PortId, &[f32])], outputs: &mut [Vec<f32>]) {
        let output = &mut outputs[0];
        drain_messages(&self.msg_receiver, |msg| self.params.handle_message(msg));
        // If we have inputs, use these buffers
        let mut signal_buf = None;
        let mut cutoff_buf = None;
        let mut resonance_buf = None;
        for (port, buffer) in inputs {
            // FRAGILE: These have to match the order the ports are declared in the UI
            match port.0 {
                0 => signal_buf = Some(buffer),
                1 => cutoff_buf = Some(buffer),
                2 => resonance_buf = Some(buffer),
                _ => {}
            }
        }
        for i in 0..output.len() {
            let signal = signal_buf
                .map(|b| b[i])
                .unwrap_or(0.0);

            let cutoff = cutoff_buf
                .map(|b| b[i])
                .unwrap_or(self.params.cutoff);

            let resonance = resonance_buf
                .map(|b| b[i])
                .unwrap_or(self.params.resonance);

            output[i] = self.step(signal, self.params.mode, cutoff, resonance);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::TAU;

    fn node(mode: FilterMode, cutoff: f32, resonance: f32) -> FilterNode {
        let (mut node, _) = FilterNode::new();
        node.params.mode = mode;
        node.params.cutoff = cutoff;
        node.params.resonance = resonance;
        node
    }

    fn sine(freq: f32, samples: usize) -> Vec<f32> {
        (0..samples).map(|i| (TAU * freq * i as f32 / SAMPLERATE as f32).sin()).collect()
    }

    fn run(node: &mut FilterNode, signal: &[f32]) -> Vec<f32> {
        let mut outputs = [vec![0.; signal.len()]];
        node.process(&[(PortId(0), signal)], &mut outputs);
        let [out] = outputs;
        out
    }

    fn rms(v: &[f32]) -> f32 {
        (v.iter().map(|x| x * x).sum::<f32>() / v.len() as f32).sqrt()
    }

    /// Gain of a filter at `freq`, measured once it has settled
    fn gain(mode: FilterMode, cutoff: f32, resonance: f32, freq: f32) -> f32 {
        let input = sine(freq, 20_000);
        let out = run(&mut node(mode, cutoff, resonance), &input);
        rms(&out[10_000..]) / rms(&input[10_000..])
    }

    #[test]
    fn low_pass_keeps_lows_and_removes_highs() {
        assert!((gain(FilterMode::LowPass, 2000., 0.707, 100.) - 1.).abs() < 0.02);
        assert!(gain(FilterMode::LowPass, 2000., 0.707, 15_000.) < 0.02);
        // half power (-3 dB) at the cutoff for a Butterworth-style Q
        assert!((gain(FilterMode::LowPass, 2000., 0.707, 2000.) - 0.707).abs() < 0.03);
    }

    #[test]
    fn high_pass_keeps_highs_and_removes_lows() {
        assert!((gain(FilterMode::HighPass, 2000., 0.707, 15_000.) - 1.).abs() < 0.05);
        assert!(gain(FilterMode::HighPass, 2000., 0.707, 100.) < 0.02);
    }

    #[test]
    fn band_pass_favours_the_cutoff() {
        let at = gain(FilterMode::BandPass, 2000., 4., 2000.);
        assert!(at > 5. * gain(FilterMode::BandPass, 2000., 4., 200.));
        assert!(at > 5. * gain(FilterMode::BandPass, 2000., 4., 15_000.));
    }

    #[test]
    fn notch_removes_the_cutoff_and_keeps_the_rest() {
        assert!(gain(FilterMode::Notch, 2000., 4., 2000.) < 0.02);
        assert!((gain(FilterMode::Notch, 2000., 4., 200.) - 1.).abs() < 0.05);
        assert!((gain(FilterMode::Notch, 2000., 4., 15_000.) - 1.).abs() < 0.05);
    }

    #[test]
    fn resonance_peaks_a_low_pass_at_the_cutoff() {
        assert!(gain(FilterMode::LowPass, 2000., 10., 2000.) > 5.);
    }

    #[test]
    fn cutoff_and_resonance_ports_override_the_knobs() {
        let input = sine(8000., 20_000);
        let cutoff = vec![200.; input.len()]; // the knob says 20 kHz: nothing would be filtered
        let resonance = vec![0.707; input.len()];
        let mut n = node(FilterMode::LowPass, 20_000., 0.707);
        let mut outputs = [vec![0.; input.len()]];
        n.process(&[(PortId(0), &input), (PortId(1), &cutoff), (PortId(2), &resonance)], &mut outputs);
        assert!(rms(&outputs[0][10_000..]) < 0.01);
    }

    #[test]
    fn nothing_connected_is_silent() {
        let (mut n, _) = FilterNode::new();
        let mut outputs = [vec![1.; 64]];
        n.process(&[], &mut outputs);
        assert!(outputs[0].iter().all(|&x| x == 0.));
    }

    #[test]
    fn absurd_settings_never_produce_nan() {
        let input = sine(440., 2_000);
        for mode in FilterMode::ALL {
            for cutoff in [0., -100., f32::NAN, 1e9, f32::INFINITY, 24_000., 1.] {
                for resonance in [0., -3., f32::NAN, 1e6, 0.001] {
                    let out = run(&mut node(mode, cutoff, resonance), &input);
                    assert!(out.iter().all(|x| x.is_finite()), "{mode:?} cutoff {cutoff} Q {resonance}");
                }
            }
        }
    }

    #[test]
    fn nan_input_doesnt_silence_the_filter_afterwards() {
        let mut n = node(FilterMode::LowPass, 2000., 0.707);
        let mut input = sine(100., 5_000);
        input[1_000] = f32::NAN;
        input[1_001] = f32::INFINITY;
        let out = run(&mut n, &input);
        assert!(out.iter().all(|x| x.is_finite()));
        assert!(rms(&out[3_000..]) > 0.5); // still passing the tone
    }

    #[test]
    fn sweeping_the_cutoff_per_sample_stays_stable() {
        let input = sine(440., 48_000);
        let cutoff: Vec<f32> = (0..input.len()).map(|i| 20. * 1000f32.powf(i as f32 / input.len() as f32)).collect();
        let resonance = vec![8.; input.len()];
        let mut n = node(FilterMode::LowPass, 1000., 1.);
        let mut outputs = [vec![0.; input.len()]];
        n.process(&[(PortId(0), &input), (PortId(1), &cutoff), (PortId(2), &resonance)], &mut outputs);
        assert!(outputs[0].iter().all(|x| x.is_finite() && x.abs() < 50.));
    }

    #[test]
    fn block_size_does_not_matter() {
        let input = sine(300., 5_000);
        let expected = run(&mut node(FilterMode::BandPass, 1000., 2.), &input);
        let mut chunked = node(FilterMode::BandPass, 1000., 2.);
        let got: Vec<f32> = input.chunks(128).flat_map(|c| run(&mut chunked, c)).collect();
        assert_eq!(got, expected);
    }
}
