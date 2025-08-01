use super::{Node, SAMPLERATE};
use std::f32::consts::PI;
use std::sync::mpsc::channel;
use std::sync::mpsc::Sender;
use std::sync::mpsc::Receiver;

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

pub struct AdsrNode {
    params: OscParameters,
    phase: f32,
    msg_receiver: Receiver<OscMessage>,
}

impl AdsrNode {
    pub fn new(n: &str) -> (Self, OscNodeHandle) {
        let (msg_sender, msg_receiver) = channel();
        let params = OscParameters {
            freq: 220.,
            vol: 32,
            name: n.to_owned(),
        };
        let handler = OscNodeHandle {
            sender: msg_sender,
            params: params.clone(), // TODO not tightly coupled...
        };
        (Self {
            params,
            phase: 0.,
            msg_receiver,
        }, handler)
    }
}

impl Node for AdsrNode {
    fn process(&mut self, output: &mut [f32]) {
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
