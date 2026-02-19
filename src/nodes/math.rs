use std::sync::mpsc::Sender;
use crate::nodes::PortDescriptions;
use crate::nodes::graph;
use super::{Node, Message, NodeUi, math};

use egui::Id;
use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;

#[derive(Debug, Clone)]
pub enum Operation {
    Add,
    Mul,
    Exp,
}

#[derive(Debug, Clone)]
pub struct MathParameters {
    pub a: f32,
    pub b: f32,
    pub sender: Sender<Message>,
    pub operation: Operation,
}

impl MathParameters {
    fn handle_message(&mut self, msg: Message) {
        match msg {
            Message::Operation(val) => self.operation = val,
            Message::SetA(val) => self.a = val,
            Message::SetB(val) => self.b = val,
            _ => (),
        }
    }
}

impl NodeUi for MathParameters {
    fn draw(&mut self, ctx: &egui::Context, idx: usize) -> (PortDescriptions, egui::Response) {
        let window = egui::Window::new("Math").id(Id::new(idx)).show(ctx, |ui| {
            ui.horizontal(|ui| {
                let op = match self.operation {
                    math::Operation::Add => "+",
                    math::Operation::Mul => "*",
                    math::Operation::Exp => "^",
                };
                ui.vertical(|ui| {
                    let a_res = ui.add_enabled(true, egui::DragValue::new(&mut self.a));
                    ui.menu_button(op, |ui| {
                        ui.set_width(100.0); // To make sure we wrap long text
                        if ui.button("+").clicked() {
                            self.operation = math::Operation::Add;
                            let _ = self.sender.send(Message::Operation(math::Operation::Add));
                        }
                        if ui.button("*").clicked() {
                            self.operation = math::Operation::Mul;
                            let _ = self.sender.send(Message::Operation(math::Operation::Mul));
                        }
                        if ui.button("^").clicked() {
                            self.operation = math::Operation::Exp;
                            let _ = self.sender.send(Message::Operation(math::Operation::Exp));
                        }
                    });
                    let b_res = ui.add_enabled(true, egui::DragValue::new(&mut self.b));
                    if a_res.changed() {
                        let _ = self.sender.send(Message::SetA(self.a));
                    }
                    if b_res.changed() {
                        let _ = self.sender.send(Message::SetB(self.b));
                    }
                });
            });
        }).unwrap();

        (PortDescriptions::with_ports(
            vec!["a", "b"],
            vec!["result"]
        ), window.response)
    }
}

pub struct MathNode {
    params: MathParameters,
    msg_receiver: Receiver<Message>,
}

impl MathNode {
    pub fn new() -> (Self, MathParameters) {
        let (msg_sender, msg_receiver) = channel();
        let params = MathParameters {
            a: 1.0,
            b: 1.0,
            sender: msg_sender,
            operation: Operation::Mul,
        };
        let handler = params.clone();
        (Self {
            params,
            msg_receiver,
        }, handler)
    }
}

impl Node for MathNode {
    fn process(&mut self, inputs: &[(graph::PortId, &[f32])], output: &mut [f32]) {
        if let Ok(msg) = self.msg_receiver.try_recv() {
            self.params.handle_message(msg);
        };
        // If we have inputs, use these buffers
        let mut a_buf = None;
        let mut b_buf = None;
        for (port, buffer) in inputs {
            // FRAGILE: These have to match the order the ports are declared in the UI
            match port.0 {
                0 => a_buf = Some(buffer),
                1 => b_buf = Some(buffer),
                _ => {}
            }
        }
        for i in 0..output.len() {
            let a = a_buf
                .map(|b| b[i])
                .unwrap_or(self.params.a);

            let b = b_buf
                .map(|b| b[i])
                .unwrap_or(self.params.b);

            match self.params.operation {
                Operation::Add => {
                    output[i] = a + b;
                },
                Operation::Mul => {
                    output[i] = a * b;
                },
                Operation::Exp => {
                    output[i] = a.powf(b);
                },
            }
        }
    }
}

#[cfg(test)]
use wasm_bindgen_test::*;
#[cfg(test)]
mod tests {
    use super::*;
    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_sine() {
        let (mut sine_osc, _) = MathNode::new();
        let mut output = [0f32; 128];
        sine_osc.process(&[], &mut output);
        assert_eq!(output[0], 0.); // TODO add more cases lol
    }
}
