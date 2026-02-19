use super::{Node, Message, graph, NodeUi, PortDescriptions, SAMPLERATE, ratio2pole};
use std::sync::mpsc::channel;
use std::sync::mpsc::Sender;
use std::sync::mpsc::Receiver;
use egui::Id;

#[derive(Debug, Clone)]
pub struct AdsrParameters {
    pub gate: bool,
    pub a: f32, // time
    pub d: f32, // time
    pub s: f32, // amplitude
    pub r: f32, // time
    pub name: String,
    pub sender: Sender<Message>,
}

enum State {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release
}

impl NodeUi for AdsrParameters {
    fn draw(&mut self, ctx: &egui::Context, idx: usize) -> (PortDescriptions, egui::Response) {
        let window = egui::Window::new("Envelope").id(Id::new(idx)).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    // TODO handle a/d/s/r messages
                    let a_res = ui.add_enabled(true, egui::DragValue::new(&mut self.a));
                    let d_res = ui.add_enabled(true, egui::DragValue::new(&mut self.d));
                    let s_res = ui.add_enabled(true, egui::DragValue::new(&mut self.s));
                    let r_res = ui.add_enabled(true, egui::DragValue::new(&mut self.r));
                    let gate_res = ui.add_enabled(true, egui::Button::new("on"));
                    if gate_res.clicked() {
                        self.gate = !self.gate; 
                        let _ = self.sender.send(Message::Gate(self.gate));
                    }
                });
            });
        }).unwrap();

        (PortDescriptions::with_ports(
            vec!["gate", "signal", "attack time", "decay time", "sustain level", "release time"],
            vec!["signal out"]
        ), window.response)
    }
}

pub struct AdsrNode {
    params: AdsrParameters,
    state: State,
    tick: u32, // how many samples we've been in the current state
    last_gate: f32, // what value last triggered an attack?
    current_out: f32, // level we are currently at
    msg_receiver: Receiver<Message>,
    inputs: Vec<Box<dyn Node>>,
}

impl AdsrNode {
    pub fn new() -> (Self, AdsrParameters) {
        let (msg_sender, msg_receiver) = channel();
        let params = AdsrParameters {
            gate: false,
            a: 0.1,
            d: 0.1,
            s: 0.8,
            r: 2.0,
            name: "ADSR".to_string(),
            sender: msg_sender,
        };
        let handler = params.clone();
        (Self {
            params,
            tick: 0,
            msg_receiver,
            last_gate: 0.,
            current_out: 0.,
            state: State::Idle,
            inputs: Vec::new(),
        }, handler)
    }
}

impl Node for AdsrNode {
    fn process(&mut self, inputs: &[(graph::PortId, &[f32])], output: &mut [f32]) {
        let msg = self.msg_receiver.try_recv();
        if let Ok(msg) = msg { if let Message::Gate(val) = msg { self.params.gate = val } };
        // If we have inputs, use these buffers
        let mut gate_buf = None;
        let mut signal_buf = None;
        for (port, buffer) in inputs {
            // FRAGILE: These have to match the order the ports are declared in the UI
            // A possible future solution will have something like PortType rather than
            // portId, but I am concerned that I might want to have one node with duplicate
            // inputs in the future, so I'm not doing that yet
            match port.0 {
                0 => gate_buf = Some(buffer),
                1 => signal_buf = Some(buffer),
                _ => {}
            }
        }
        for i in 0..output.len() {
            let gate = gate_buf
                .map(|b| b[i])
                .unwrap_or(if self.params.gate {1.} else {0.0});

            let signal = signal_buf
                .map(|b| b[i])
                .unwrap_or(0.0);

            if gate > self.last_gate {
                self.tick = 0;
                self.state = State::Attack;
            }
            else if gate == 0.0 {
                self.tick = 0;
                self.state = State::Release;
            }

            let mut target = 0.0;
            let mut pole = 0.0;

            match self.state {
                State::Idle => {
                    self.current_out = 0.0;
                },
                State::Attack => {
                    self.tick += 1;
                    if self.tick as f32 / SAMPLERATE as f32 >= self.params.a {
                        self.tick = 0;
                        self.state = State::Decay;
                    }
                    target = 1.+f32::EPSILON;
                    pole = ratio2pole(self.params.a, f32::EPSILON/target);
                },
                State::Decay => {
                    self.tick += 1;
                    if self.tick as f32 / SAMPLERATE as f32 >= self.params.d {
                        self.state = State::Sustain;
                    }
                    pole = ratio2pole(self.params.d, f32::EPSILON/(1.-self.params.s+f32::EPSILON));
                },
                State::Sustain => {
                    self.current_out = self.params.s;
                },
                State::Release => {
                    self.tick += 1;
                    self.last_gate = 0.0;
                    target = -f32::EPSILON;
                    pole = ratio2pole(self.params.r, f32::EPSILON/(self.params.s+f32::EPSILON));
                }
            };
            self.current_out = (1.-pole)*target + pole*self.current_out;
            output[i] = signal * self.current_out;

            // output[i] = (phase * TAU).sin() * (vol / 100.0);
            // self.phase = phase.rem_euclid(1.);
        }
    }
}

impl AdsrNode {
    fn tick(gate: f32) {
    }
}
