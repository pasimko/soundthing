// Wasm audio processors can be implemented in Rust without knowing
// about audio worklets.


use super::Node;
use super::NodeUi;
use std::f32::consts::PI;

pub struct SineOsc {
    frequency: u32,
    volume: u8,
    phase: f32,
    name: String,
}

impl SineOsc {
    pub fn new(n: &str) -> Self {
        Self {
            frequency: 128,
            volume: 128,
            phase: 0.,
            name: n.to_owned(),
        }
    }
}

// TODO i think this is slightly inaccurate - I can hear < 20hz
// TODO actually check sample rate
impl Node for SineOsc {
    fn process(&mut self, output: &mut [f32]) -> bool {
        for a in output {
            let frequency = self.frequency;
            let volume = self.volume;
            self.phase += 2. * PI / (48_000. / frequency as f32);
            self.phase = self.phase.rem_euclid(2. * PI);
            *a = (self.phase).sin() * (volume as f32 / 100.);
        }
        true
    }
}

impl NodeUi for SineOsc {
    fn build_controls(&mut self, ctx: &egui::Context) {
        egui::Window::new(self.name.as_str()).show(ctx, |ui| {
            ui.add(egui::Slider::new(&mut self.frequency, 20..=2000).text("frequency").logarithmic(true));
            ui.add(egui::Slider::new(&mut self.volume, 0..=128).text("volume"));
        });
    }
}

pub struct SawtoothOsc {
    frequency: u32,
    volume: u8,
    phase: f32,
}

impl SawtoothOsc {
    pub fn new() -> Self {
        Self {
            frequency: 128,
            volume: 128,
            phase: 0.,
        }
    }
}

impl Node for SawtoothOsc {
    fn process(&mut self, output: &mut [f32]) -> bool {
        for a in output {
            let frequency = self.frequency;
            let volume = self.volume;
            self.phase += frequency as f32 / 48_000.;
            self.phase = self.phase.rem_euclid(2.);
            *a = (self.phase - 1.) * (volume as f32 / 100.);
        }
        true
    }
}

impl NodeUi for SawtoothOsc {
    fn build_controls(&mut self, ctx: &egui::Context) {
        egui::Window::new("Sawtooth Osc").show(ctx, |ui| {
            ui.add(egui::Slider::new(&mut self.frequency, 1..=440).text("frequency"));
            ui.add(egui::Slider::new(&mut self.volume, 0..=128).text("volume"));
        });
    }
}
