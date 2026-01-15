use super::{Node, Message, graph, NodeUi, PortResponses};
use std::sync::mpsc::channel;
use std::sync::mpsc::Sender;
use std::sync::mpsc::Receiver;
use egui::Id;

#[derive(Debug, Clone)]
pub struct AdsrParameters {
    pub on: bool,
    pub name: String,
    pub sender: Sender<Message>,
}

impl NodeUi for AdsrParameters {
    fn draw(&mut self, ctx: &egui::Context, idx: usize) -> PortResponses {
        let mut port_responses = PortResponses::new();
        egui::Window::new("ADSR").id(Id::new(idx)).show(ctx, |ui| {
            ui.horizontal(|ui| {
                // port buttons
                ui.vertical(|ui| {
                    let gate_button_response = ui.add(egui::Button::new("on"));
                    port_responses.inputs.push(gate_button_response);
                });
                // ui.vertical(|ui| {
                //     ui.set_width(100.0); // To make sure we wrap long text
                //     let bpm_res = ui.add_enabled(true,
                //         egui::Slider::new(&mut self.bpm, 0.1..=2000.0)
                //         .text("bpm")
                //         .logarithmic(true)
                //     );
                //     if bpm_res.changed() {
                //         let _ = self.sender.send(Message::Bpm(self.bpm));
                //     }
                // });
                // output ports
                ui.vertical(|ui| {
                    let out_button_response = ui.add(egui::Button::new("out"));
                    port_responses.outputs.push(out_button_response);
                });
            });
        });
        port_responses
    }
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
    fn process(&mut self, inputs: &[(graph::PortId, &[f32])], output: &mut [f32]) {
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
