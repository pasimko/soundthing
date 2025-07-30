use super::{Node, SAMPLERATE};
use std::f32::consts::PI;
use std::sync::mpsc::channel;
use std::sync::mpsc::Sender;
use std::sync::mpsc::Receiver;

// We only update one parameter at a time, but maybe code would
// be cleaner if we sent a struct in the channel
// TODO check the performance of that
// I'll do it the stupid way first
pub enum OscMessage {
    Frequency(f32),
    Volume(u8),
}

#[derive(Default, Debug, Clone)]
pub struct OscParameters {
    pub freq: f32,
    pub vol: u8,
    pub name: String,
}


pub struct OscNodeHandle {
    pub sender: Sender<OscMessage>,
    pub params: OscParameters,
}

pub struct SineOsc {
    params: OscParameters,
    phase: f32,
    name: String,
    msg_receiver: Receiver<OscMessage>,
    // Need to figure out how we call split
    // writer: HeapProd<f32>,
}

impl SineOsc {
    // TODO idk if returning 
    pub fn new(n: &str) -> (Self, OscNodeHandle) {
        // let ring_buffer = HeapRb::<f32>::new(BUFSIZE);
        // let (writer, reader) = ring_buffer.split();
        let (msg_sender, msg_receiver) = channel();
        let params = OscParameters {
            freq: 220.,
            vol: 32,
            name: String::from("Sine Osc"),
        };
        let handler = OscNodeHandle {
            sender: msg_sender,
            params: params.clone(), // TODO not tightly coupled...
        };
        (Self {
            params,
            phase: 0.,
            name: n.to_owned(), // egui/eframe has a glitch??
            msg_receiver,
        }, handler)
    }
}

// TODO
// this sounds crunchy except for:
// 500, 380, 750 -- ?
impl Node for SineOsc {
    fn process(&mut self, output: &mut [f32]) {
        // need to read all the sent messages somehow... hmm
        let msg = self.msg_receiver.try_recv();
        if let Ok(msg) = msg { match msg {
            OscMessage::Frequency(val) => self.params.freq = val as f32,
            OscMessage::Volume(val) => self.params.vol = val,
        } };
        for sample in output.iter_mut() {
            let frequency = self.params.freq;
            let volume = self.params.vol;
            self.phase += 2. * PI / (SAMPLERATE as f32 / frequency);
            self.phase = self.phase.rem_euclid(2. * PI);
            *sample = (self.phase).sin() * (volume as f32 / 100.);
        }
    }
}

// pub struct SawtoothOsc {
//     frequency: u32,
//     volume: u8,
//     phase: f32,
//     name: String,
//     msg_receiver: Receiver<OscMessage>,
//     // Need to figure out how we call split
//     // writer: HeapProd<f32>,
// }
// 
// impl SawtoothOsc {
//     pub fn new(n: &str) -> (Self, Sender<OscMessage>) {
//         // let ring_buffer = HeapRb::<f32>::new(BUFSIZE);
//         // let (writer, reader) = ring_buffer.split();
//         let (msg_sender, msg_receiver) = channel();
//         (Self {
//             frequency: 128,
//             volume: 128,
//             phase: 0.,
//             name: n.to_owned(), // egui/eframe has a glitch??
//             msg_receiver,
//         }, msg_sender)
//     }
// }
// 
// impl Node for SawtoothOsc {
//     fn process(&mut self, output: &mut [f32]) {
//         let msg = self.msg_receiver.try_recv();
//         if let Ok(msg) = msg { match msg {
//             OscMessage::Frequency(val) => self.frequency = val,
//             OscMessage::Volume(val) => self.volume = val,
//         } };
//         for a in output {
//             let frequency = self.frequency;
//             let volume = self.volume;
//             self.phase += frequency as f32 / 48_000.;
//             self.phase = self.phase.rem_euclid(2.);
//             *a = (self.phase - 1.) * (volume as f32 / 100.);
//         }
//     }
// }
// 
// pub struct SquareOsc {
//     frequency: u32,
//     volume: u8,
//     phase: f32,
//     name: String,
//     msg_receiver: Receiver<OscMessage>,
//     // Need to figure out how we call split
//     // writer: HeapProd<f32>,
// }
// 
// impl SquareOsc {
//     pub fn new(n: &str) -> (Self, Sender<OscMessage>) {
//         // let ring_buffer = HeapRb::<f32>::new(BUFSIZE);
//         // let (writer, reader) = ring_buffer.split();
//         let (msg_sender, msg_receiver) = channel();
//         (Self {
//             frequency: 128,
//             volume: 128,
//             phase: 0.,
//             name: n.to_owned(), // egui/eframe has a glitch??
//             msg_receiver,
//         }, msg_sender)
//     }
// }
// 
// impl Node for SquareOsc {
//     fn process(&mut self, output: &mut [f32]) {
//         let msg = self.msg_receiver.try_recv();
//         if let Ok(msg) = msg { match msg {
//             OscMessage::Frequency(val) => self.frequency = val,
//             OscMessage::Volume(val) => self.volume = val,
//         } };
//         for a in output {
//             let frequency = self.frequency;
//             let volume = self.volume;
//             self.phase += frequency as f32 / 48_000.;
//             self.phase = self.phase.rem_euclid(1.);
//             *a = self.phase.round() * (volume as f32 / 100.);
//         }
//     }
// }
