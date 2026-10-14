use std::sync::mpsc::Sender;
use crate::nodes::PortDescriptions;
use crate::nodes::graph::PortId;
use super::{PortInfo, Node, SAMPLERATE, NodeUi, drain_messages, ACTIVITY_REPAINT_SECS};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;

#[derive(Debug, Clone)]
pub enum Operation {
    Add,
    Mul,
    Exp,
}

// Length of each output's note, in beats (quarter notes), in output port order.
const NOTE_BEATS: [f64; 5] = [4.0, 2.0, 1.0, 0.5, 0.25];
// The longest note; the beat counter wraps here so all outputs stay aligned.
const BAR_BEATS: f64 = 4.0;
// The output the beat light follows: the quarter note
const BEAT_OUTPUT: usize = 2;
const FLASH_SECS: f64 = 0.15;

pub enum MetronomeMessage {
    Bpm(f32),
}

#[derive(Debug, Clone)]
pub struct MetronomeParameters {
    pub bpm: f32,
    pub sender: Sender<MetronomeMessage>,
    // Beats counted by the audio side. The UI flashes whenever it changes.
    pub beats: Arc<AtomicU32>,
    last_beat: u32,
    flash_started: f64,
}

impl MetronomeParameters {
    fn handle_message(&mut self, msg: MetronomeMessage) {
        match msg {
            MetronomeMessage::Bpm(val) => self.bpm = val,
        }
    }
}

impl NodeUi for MetronomeParameters {
    fn duplicate(&self) -> Option<(Box<dyn Node>, Box<dyn NodeUi>)> {
        let (node, params) = MetronomeNode::from_params(self.clone());
        Some((Box::new(node), Box::new(params)))
    }

    fn title(&self) -> String {
        "Metronome".to_string()
    }

    fn ports(&self) -> PortDescriptions {
        PortDescriptions::with_ports(
            vec![
                PortInfo::input("bpm", "Tempo in beats (quarter notes) per minute. Overrides the slider."),
            ],
            vec![
                PortInfo::output("whole", "1 for a single sample every four beats, otherwise 0."),
                PortInfo::output("half", "1 for a single sample every two beats, otherwise 0."),
                PortInfo::output("quarter", "1 for a single sample on every beat, otherwise 0."),
                PortInfo::output("eighth", "1 for a single sample twice per beat, otherwise 0."),
                PortInfo::output("sixteenth", "1 for a single sample four times per beat, otherwise 0."),
            ]
        )
    }

    fn draw(&mut self, ui: &mut egui::Ui) {
        let now = ui.input(|i| i.time);
        let beats = self.beats.load(Ordering::Relaxed);
        if beats != self.last_beat {
            self.last_beat = beats;
            self.flash_started = now;
        }
        let flash = (1.0 - (now - self.flash_started) / FLASH_SECS).clamp(0.0, 1.0) as f32;
        ui.ctx().request_repaint_after_secs(ACTIVITY_REPAINT_SECS);

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_width(100.0);
                let bpm_res = ui.add_enabled(true,
                    egui::Slider::new(&mut self.bpm, 0.1..=2000.0)
                    .text("bpm")
                    .logarithmic(true)
                );
                if bpm_res.changed() {
                    let _ = self.sender.send(MetronomeMessage::Bpm(self.bpm));
                }
            });
            let (light, _) = ui.allocate_exact_size(egui::Vec2::splat(24.0), egui::Sense::hover());
            ui.painter().circle(
                light.center(),
                10.0,
                ui.visuals().selection.bg_fill.gamma_multiply(flash),
                egui::Stroke::new(1.5_f32, ui.visuals().text_color()),
            );
        });
    }
}

pub struct MetronomeNode {
    params: MetronomeParameters,
    // position within the bar, in beats; f64 so the tempo doesn't drift
    beats: f64,
    msg_receiver: Receiver<MetronomeMessage>,
}

impl MetronomeNode {
    pub fn new() -> (Self, MetronomeParameters) {
        let (msg_sender, msg_receiver) = channel();
        let params = MetronomeParameters {
            bpm: 120.0,
            sender: msg_sender,
            beats: Arc::new(AtomicU32::new(0)),
            last_beat: 0,
            flash_started: f64::NEG_INFINITY,
        };
        Self::build(params, msg_receiver)
    }

    pub fn from_params(mut params: MetronomeParameters) -> (Self, MetronomeParameters) {
        let (msg_sender, msg_receiver) = channel();
        params.sender = msg_sender;
        // a copy keeps its own beat count rather than flashing along with the original
        params.beats = Arc::new(AtomicU32::new(0));
        params.last_beat = 0;
        params.flash_started = f64::NEG_INFINITY;
        Self::build(params, msg_receiver)
    }

    fn build(params: MetronomeParameters, msg_receiver: Receiver<MetronomeMessage>) -> (Self, MetronomeParameters) {
        let handler = params.clone();
        (Self {
            params,
            beats: 0.0,
            msg_receiver,
        }, handler)
    }
}

impl Node for MetronomeNode {
    fn output_count(&self) -> usize {
        NOTE_BEATS.len()
    }

    fn process(&mut self, inputs: &[(PortId, &[f32])], outputs: &mut [Vec<f32>]) {
        drain_messages(&self.msg_receiver, |msg| self.params.handle_message(msg));
        // If we have inputs, use these buffers
        let mut bpm_buf = None;
        for (port, buffer) in inputs {
            // FRAGILE: These have to match the order the ports are declared in the UI
            if port.0 == 0 { bpm_buf = Some(buffer) }
        }
        let mut beats = 0;
        for i in 0..outputs[0].len() {
            let bpm = bpm_buf
                .map(|b| b[i])
                .unwrap_or(self.params.bpm);

            // `max` keeps the clock from running backwards (and turns NaN into a stopped clock)
            let before = self.beats;
            let after = before + bpm.max(0.) as f64 / 60. / SAMPLERATE as f64;

            // A note's pulse fires on the sample where the clock crosses a multiple of its length
            for (output, note) in outputs.iter_mut().zip(NOTE_BEATS) {
                output[i] = if (after / note).floor() > (before / note).floor() { 1.0 } else { 0.0 };
            }
            self.beats = after.rem_euclid(BAR_BEATS);
            if outputs[BEAT_OUTPUT][i] == 1.0 {
                beats += 1;
            }
        }
        if beats > 0 {
            self.params.beats.fetch_add(beats, Ordering::Relaxed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(node: &mut MetronomeNode, samples: usize) {
        let mut outputs = vec![vec![0.; samples]; NOTE_BEATS.len()];
        node.process(&[], &mut outputs);
    }

    #[test]
    fn counts_one_beat_per_quarter_note_pulse() {
        // 120 bpm is two beats a second
        let (mut node, params) = MetronomeNode::new();
        run(&mut node, SAMPLERATE + 100);
        assert_eq!(params.beats.load(Ordering::Relaxed), 2);
        run(&mut node, SAMPLERATE);
        assert_eq!(params.beats.load(Ordering::Relaxed), 4);
    }

    #[test]
    fn a_duplicate_counts_separately() {
        let (_, params) = MetronomeNode::new();
        let (mut copy, copy_params) = MetronomeNode::from_params(params.clone());
        run(&mut copy, SAMPLERATE + 100);
        assert_eq!(params.beats.load(Ordering::Relaxed), 0);
        assert_eq!(copy_params.beats.load(Ordering::Relaxed), 2);
    }
}
