use super::{Node, Message, graph};
use std::sync::mpsc::channel;
use std::sync::mpsc::Sender;
use std::sync::mpsc::Receiver;

#[derive(Debug, Clone)]
pub struct AdsrParameters {
    pub on: bool,
    pub name: String,
    pub sender: Sender<Message>,
}

pub struct AdsrNode {
    params: AdsrParameters,
    msg_receiver: Receiver<Message>,
    inputs: Vec<Box<dyn Node>>,
    on_count: u8,
}

impl AdsrNode {
    pub fn new() -> (Self, AdsrParameters) {
        let (msg_sender, msg_receiver) = channel();
        let params = AdsrParameters {
            on: false,
            name: "ADSR".to_string(),
            sender: msg_sender,
        };
        let handler = params.clone();
        (Self {
            params,
            msg_receiver,
            inputs: Vec::new(),
            on_count: 0,
        }, handler)
    }
}

impl Node for AdsrNode {
    fn process(&mut self, inputs: &[(graph::Port, &[f32])], output: &mut [f32]) {
        let msg = self.msg_receiver.try_recv();
        if let Ok(msg) = msg { if let Message::On(val) = msg { self.params.on = val } };
        if self.params.on {
            output.iter_mut().for_each( |x| *x = 0.);
            for idx in 0..output.len() {
                for input in inputs {
                    output[idx] += input.1[idx];
                }
            }
            if self.on_count < 100 {
                self.on_count += 1;
            }
            else {
                self.on_count = 0;
            }
        }
    }
}
