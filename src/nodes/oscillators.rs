// Wasm audio processors can be implemented in Rust without knowing
// about audio worklets.

use super::Node;
use super::NodeUi;
use std::f32::consts::PI;
use std::sync::mpsc::channel;
use std::sync::mpsc::Sender;
use std::sync::mpsc::Receiver;

pub struct SineOsc {
    frequency: u32,
    volume: u8,
    phase: f32,
    name: String,
    msg_channel: (Sender<u32>, Receiver<u32>),
}

impl SineOsc {
    pub fn new(n: &str) -> Self {
        Self {
            frequency: 128,
            volume: 128,
            phase: 0.,
            name: n.to_owned(),
            msg_channel: channel(),
        }
    }
}
// TODO i think this is slightly inaccurate - I can hear < 20hz
// TODO actually check sample rate
impl Node for SineOsc {
    fn process(&mut self, _phase: u32, output: &mut [f32]) -> bool {
        for a in output {
            let frequency = match self.msg_channel.1.try_recv() {
                Ok(val) => val,
                _ => self.frequency,
            };
            // let frequency = self.frequency;
            let volume = self.volume;
            self.phase += 2. * PI / (48_000. / frequency as f32);
            self.phase = self.phase.rem_euclid(2. * PI);
            *a = (self.phase).sin() * (volume as f32 / 100.);
        }
        true
    }
    fn build_controls(&self, ctx: &egui::Context) {
        egui::Window::new(self.name.as_str()).show(ctx, |ui| {
            let mut new_freq = self.frequency;
            let mut new_vol = self.volume;
            ui.add(egui::Slider::new(&mut new_freq, 20..=2000).text("frequency").logarithmic(true));
            ui.add(egui::Slider::new(&mut new_vol, 0..=128).text("volume"));
            self.msg_channel.0.send(new_freq).unwrap();
        });
    }
}

// pub struct SawtoothOsc {
//     frequency: u32,
//     volume: u8,
//     phase: f32,
// }
// 
// impl SawtoothOsc {
//     pub fn new() -> Self {
//         Self {
//             frequency: 128,
//             volume: 128,
//             phase: 0.,
//         }
//     }
// }
// 
// impl Node for SawtoothOsc {
//     fn process(&mut self, phase: u32, output: &mut [f32]) -> bool {
//         for a in output {
//             let frequency = self.frequency;
//             let volume = self.volume;
//             self.phase += frequency as f32 / 48_000.;
//             self.phase = self.phase.rem_euclid(2.);
//             *a = (self.phase - 1.) * (volume as f32 / 100.);
//         }
//         true
//     }
// }
// 
// impl NodeUi for SawtoothOsc {
//     fn build_controls(&mut self, ctx: &egui::Context) {
//         egui::Window::new("Sawtooth Osc").show(ctx, |ui| {
//             ui.add(egui::Slider::new(&mut self.frequency, 1..=440).text("frequency"));
//             ui.add(egui::Slider::new(&mut self.volume, 0..=128).text("volume"));
//         });
//     }
// }
// 
// pub struct SquareOsc {
//     frequency: u32,
//     volume: u8,
//     phase: f32,
// }
// 
// impl SquareOsc {
//     pub fn new() -> Self {
//         Self {
//             frequency: 128,
//             volume: 128,
//             phase: 0.,
//         }
//     }
// }
// 
// impl Node for SquareOsc {
//     fn process(&mut self, phase: u32, output: &mut [f32]) -> bool {
//         for a in output {
//             let frequency = self.frequency;
//             let volume = self.volume;
//             self.phase += frequency as f32 / 48_000.;
//             self.phase = self.phase.rem_euclid(1.);
//             *a = self.phase.round() * (volume as f32 / 100.);
//         }
//         true
//     }
// }
// 
// // TODO make this work with message passing
// impl NodeUi for SquareOsc {
//     fn build_controls(&self, ctx: &egui::Context) {
//         egui::Window::new("Square Osc").show(ctx, |ui| {
//             ui.add(egui::Slider::new(&mut self.frequency, 1..=440).text("frequency"));
//             ui.add(egui::Slider::new(&mut self.volume, 0..=128).text("volume"));
//         });
//     }
// }
