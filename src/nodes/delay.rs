use std::sync::mpsc::Sender;
use ringbuf::{traits::*,LocalRb, storage::Heap};
use crate::nodes::PortDescriptions;
use crate::nodes::graph::PortId;
use super::{Node, SAMPLERATE, Message, NodeUi};
use egui::Id;

use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;

#[derive(Debug, Clone)]
pub struct DelayParameters {
    pub delay: u32,
    pub sender: Sender<Message>,
}

impl DelayParameters {
    fn handle_message(&mut self, msg: Message) {
        if let Message::Delay(val) = msg {
            self.delay = val;
            // this is bothering me now...
            // I'd like to be able to change the node itself in here
        }
    }
}

impl NodeUi for DelayParameters {
    fn draw(&mut self, ctx: &egui::Context, idx: usize) -> (PortDescriptions, egui::Response) {
        let window = egui::Window::new("Delay").id(Id::new(idx)).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(100.0);
                    let delay_res = ui.add_enabled(true,
                        egui::Slider::new(&mut self.delay, 1..=2000)
                        .text("delay")
                        .logarithmic(true)
                    );
                    if delay_res.changed() {
                        let _ = self.sender.send(Message::Delay(self.delay));
                    }
                });
            });
        }).unwrap();
        (PortDescriptions::with_ports(
            vec!["delay"],
            vec!["signal out"]
        ), window.response)
    }
}

pub struct DelayNode {
    params: DelayParameters,
    phase: f32,
    msg_receiver: Receiver<Message>,
    buffer: LocalRb<Heap<f32>>,
}

impl DelayNode {
    pub fn new() -> (Self, DelayParameters) {
        let (msg_sender, msg_receiver) = channel();
        let params = DelayParameters {
            delay: 500,
            sender: msg_sender,
        };
        let handler = params.clone();
        let bufsize = (SAMPLERATE as u32 / 1000 * params.delay) as usize;
        let mut buffer = LocalRb::<Heap<f32>>::new(bufsize);
        buffer.push_slice(&vec![0.; bufsize]);
        (Self {
            params,
            phase: 0.0,
            msg_receiver,
            buffer,
        }, handler)
    }
}

impl Node for DelayNode {
    fn process(&mut self, inputs: &[(PortId, &[f32])], output: &mut [f32]) {
        if let Ok(msg) = self.msg_receiver.try_recv() {
            self.params.handle_message(msg);
        };
        // Write to ringbuf
        for (port, buffer) in inputs {
            // FRAGILE: These have to match the order the ports are declared in the UI
            if port.0 == 0 {
                self.buffer.push_slice(buffer);
                // Read from ringbuf
                let mut i = 0;
                let delay_buf_iter = self.buffer.pop_iter();
                for v in delay_buf_iter.take(output.len()) {
                    output[i] = v;
                    i += 1;
                }
            }
        }
    }
}
