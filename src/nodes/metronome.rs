use std::sync::mpsc::Sender;
use crate::nodes::vol_smooth;
use crate::nodes::graph;

use super::{Node, SAMPLERATE, Message};
use std::f32::consts::PI;
use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;

#[derive(Debug, Clone)]
pub enum Operation {
    Add,
    Mul,
    Exp,
}

#[derive(Debug, Clone)]
pub struct MetronomeParameters {
    pub bpm: f32,
    pub sender: Sender<Message>,
}

impl MetronomeParameters {
    fn handle_message(&mut self, msg: Message) {
        match msg {
            Message::Bpm(val) => self.bpm = val,
            _ => (),
        }
    }
}

pub struct MetronomeNode {
    params: MetronomeParameters,
    phase: f32,
    msg_receiver: Receiver<Message>,
}

impl MetronomeNode {
    pub fn new() -> (Self, MetronomeParameters) {
        let (msg_sender, msg_receiver) = channel();
        let params = MetronomeParameters {
            bpm: 120.0,
            sender: msg_sender,
        };
        let handler = params.clone();
        (Self {
            params,
            phase: 0.0,
            msg_receiver,
        }, handler)
    }
}

impl Node for MetronomeNode {
    fn process(&mut self, inputs: &[(graph::PortId, &[f32])], output: &mut [f32]) {
        if let Ok(msg) = self.msg_receiver.try_recv() {
            self.params.handle_message(msg);
        };
        // If we have inputs, use these buffers
        let mut bpm_buf = None;
        for (port, buffer) in inputs {
            // FRAGILE: These have to match the order the ports are declared in the UI
            match port.0 {
                0 => bpm_buf = Some(buffer),
                _ => {}
            }
        }
        for i in 0..output.len() {
            let bpm = bpm_buf
                .map(|b| b[i])
                .unwrap_or(self.params.bpm);

            self.phase += bpm / SAMPLERATE as f32 / 60.;

            // this is wasm we branch in this muthafucka
            // better take yo sensitive ass back to x86
            if self.phase >= 1.0 {
                output[i] = 1.0;
                self.phase = self.phase.rem_euclid(1.);
            }
            else {
                output[i] = 0.0;
            }
        }
    }
}
