// Wasm audio processors can be implemented in Rust without knowing
// about audio worklets.

use super::{Node, SAMPLERATE};
use std::f32::consts::PI;
use std::sync::mpsc::channel;
use std::sync::mpsc::Sender;
use std::sync::mpsc::Receiver;
// use ringbuf::{HeapRb, HeapProd, HeapCons, traits::*};

pub enum OscMessage {
    Frequency(u32),
    Volume(u8),
}

pub struct SineOsc {
    frequency: u32,
    volume: u8,
    phase: f32,
    name: String,
    msg_receiver: Receiver<OscMessage>,
    // Need to figure out how we call split
    // writer: HeapProd<f32>,
}

impl SineOsc {
    // TODO idk if returning 
    pub fn new(n: &str) -> (Self, Sender<OscMessage>) {
        // let ring_buffer = HeapRb::<f32>::new(BUFSIZE);
        // let (writer, reader) = ring_buffer.split();
        let (msg_sender, msg_receiver) = channel();
        (Self {
            frequency: 128,
            volume: 128,
            phase: 0.,
            name: n.to_owned(), // egui/eframe has a glitch??
            msg_receiver,
        }, msg_sender)
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
            OscMessage::Frequency(val) => self.frequency = val,
            OscMessage::Volume(val) => self.volume = val,
        } };
        for sample in output.iter_mut() {
            // let frequency = match self.msg_channel.1.try_recv() {
            //     Ok(val) => val,
            //     _ => self.frequency,
            // };
            let frequency = self.frequency;
            let volume = self.volume;
            self.phase += 2. * PI / (SAMPLERATE as f32 / frequency as f32);
            self.phase = self.phase.rem_euclid(2. * PI);
            *sample = (self.phase).sin() * (volume as f32 / 100.);
        }
    }
}

pub struct SawtoothOsc {
    frequency: u32,
    volume: u8,
    phase: f32,
    name: String,
    msg_receiver: Receiver<OscMessage>,
    // Need to figure out how we call split
    // writer: HeapProd<f32>,
}

impl SawtoothOsc {
    pub fn new(n: &str) -> (Self, Sender<OscMessage>) {
        // let ring_buffer = HeapRb::<f32>::new(BUFSIZE);
        // let (writer, reader) = ring_buffer.split();
        let (msg_sender, msg_receiver) = channel();
        (Self {
            frequency: 128,
            volume: 128,
            phase: 0.,
            name: n.to_owned(), // egui/eframe has a glitch??
            msg_receiver,
        }, msg_sender)
    }
}

impl Node for SawtoothOsc {
    fn process(&mut self, output: &mut [f32]) {
        let msg = self.msg_receiver.try_recv();
        if let Ok(msg) = msg { match msg {
            OscMessage::Frequency(val) => self.frequency = val,
            OscMessage::Volume(val) => self.volume = val,
        } };
        for a in output {
            let frequency = self.frequency;
            let volume = self.volume;
            self.phase += frequency as f32 / 48_000.;
            self.phase = self.phase.rem_euclid(2.);
            *a = (self.phase - 1.) * (volume as f32 / 100.);
        }
    }
}

pub struct SquareOsc {
    frequency: u32,
    volume: u8,
    phase: f32,
    name: String,
    msg_receiver: Receiver<OscMessage>,
    // Need to figure out how we call split
    // writer: HeapProd<f32>,
}

impl SquareOsc {
    pub fn new(n: &str) -> (Self, Sender<OscMessage>) {
        // let ring_buffer = HeapRb::<f32>::new(BUFSIZE);
        // let (writer, reader) = ring_buffer.split();
        let (msg_sender, msg_receiver) = channel();
        (Self {
            frequency: 128,
            volume: 128,
            phase: 0.,
            name: n.to_owned(), // egui/eframe has a glitch??
            msg_receiver,
        }, msg_sender)
    }
}

impl Node for SquareOsc {
    fn process(&mut self, output: &mut [f32]) {
        let msg = self.msg_receiver.try_recv();
        if let Ok(msg) = msg { match msg {
            OscMessage::Frequency(val) => self.frequency = val,
            OscMessage::Volume(val) => self.volume = val,
        } };
        for a in output {
            let frequency = self.frequency;
            let volume = self.volume;
            self.phase += frequency as f32 / 48_000.;
            self.phase = self.phase.rem_euclid(1.);
            *a = self.phase.round() * (volume as f32 / 100.);
        }
    }
}
