use std::sync::mpsc::Sender;
use crate::nodes::graph::PortId;

use super::{Node, SAMPLERATE, Message, NodeUi, PortResponses};
use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;

use egui::Id;

#[derive(Debug, Clone)]
pub enum Waveform {
    Sine,
    Square,
    Tri,
    Saw,
}
use super::graph;

use egui::{
    Color32, Pos2, Rect, Sense, Shape, Stroke,
    emath,
    pos2,
};

#[derive(Debug, Clone)]
pub struct PhasorParameters {
    pub freq: f32,
    pub point: (f32, f32),
    pub sender: Sender<Message>,
    pub name: String,
}

impl PhasorParameters {
    fn handle_message(&mut self, msg: Message) {
        match msg {
            Message::SetA(val) => self.point.0 = val,
            Message::SetB(val) => self.point.1 = val,
            Message::Frequency(val) => self.freq = val,
            _ => (),
        }
    }
}

impl NodeUi for PhasorParameters {
    fn draw(&mut self, ctx: &egui::Context, idx: usize) -> PortResponses {
        let mut port_responses = PortResponses::new();
        egui::Window::new("Phase Bender").id(Id::new(idx)).show(ctx, |ui| {
            ui.horizontal(|ui| {
                // input ports
                ui.vertical(|ui| {
                    let x_button_response = ui.add(egui::Button::new("x"));
                    port_responses.inputs.push(x_button_response);
                    let y_button_response = ui.add(egui::Button::new("y"));
                    port_responses.inputs.push(y_button_response);
                    let freq_button_response = ui.add(egui::Button::new("freq"));
                    port_responses.inputs.push(freq_button_response);
                });
                // sliders, other non-port UI stuff
                ui.vertical(|ui| {
                    ui.set_max_width(200.0);
                    ui.set_max_height(200.0);
                    let (mut response, painter) =
                        ui.allocate_painter(ui.available_size_before_wrap(), Sense::drag());

                    let to_screen = emath::RectTransform::from_to(
                        Rect::from_min_size(Pos2::ZERO, response.rect.square_proportions()),
                        response.rect,
                    );
                    let from_screen = to_screen.inverse();

                    if let Some(pointer_pos) = response.interact_pointer_pos() {
                        let canvas_pos = from_screen * pointer_pos;
                        self.point.0 = canvas_pos[0];
                        self.point.1 = canvas_pos[1];
                        self.sender.send(Message::SetA(self.point.0)).unwrap();
                        self.sender.send(Message::SetB(self.point.1)).unwrap();
                        response.mark_changed();
                    }

                    // Draw phase bender graph
                    let lines = [vec![pos2(0., 1.), pos2(self.point.0, self.point.1), pos2(1., 0.)]];
                    let shapes = lines
                        .iter()
                        .filter(|line| line.len() >= 2)
                        .map(|line| {
                            let points: Vec<Pos2> = line.iter().map(|p| to_screen * *p).collect();
                            egui::Shape::line(points, Stroke::new(2.0, Color32::BLACK))
                        });
                    painter.extend(shapes);
                    let freq_res = ui.add(egui::Slider::new(&mut self.freq, 20.0..=2000.0).text("frequency").logarithmic(true));
                    if freq_res.changed() {
                        self.sender.send(Message::Frequency(self.freq)).unwrap();
                    }
                });

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

pub struct PhaseBender {
    params: PhasorParameters,
    phase: f32,
    freq: f32,
    msg_receiver: Receiver<Message>,
    pub point: (f32, f32),
}

impl PhaseBender {
    pub fn new() -> (Self, PhasorParameters) {
        let (msg_sender, msg_receiver) = channel();
        let point = (0.5, 0.5);
        let freq = 440.;
        let params = PhasorParameters {
            freq,
            sender: msg_sender,
            name: "Phasor".to_string(),
            point,
        };
        let handler = params.clone();
        (Self {
            params,
            freq,
            phase: 0.,
            msg_receiver,
            point,
        }, handler)
    }
}

impl Node for PhaseBender {
    fn process(&mut self, inputs: &[(graph::PortId, &[f32])], output: &mut [f32]) {
        if let Ok(msg) = self.msg_receiver.try_recv() {
            self.params.handle_message(msg);
        };
        // If we have inputs, use these buffers
        let mut x_buf = None;
        let mut y_buf = None;
        let mut freq_buf = None;
        for (port, buffer) in inputs {
            // FRAGILE: These have to match the order the ports are declared in the UI
            match port.0 {
                0 => x_buf = Some(buffer),
                1 => y_buf = Some(buffer),
                2 => freq_buf = Some(buffer),
                _ => {}
            }
        }
        for i in 0..output.len() {
            let x = x_buf
                .map(|b| b[i])
                .unwrap_or(self.params.point.0);
            let y = y_buf
                .map(|b| b[i])
                .unwrap_or(self.params.point.1 as f32);
            let freq = freq_buf
                .map(|b| b[i])
                .unwrap_or(self.params.freq);

            if self.phase < x {
                self.phase += y / x / (SAMPLERATE as f32 / freq);
            }
            else {
                self.phase += (1. - y) / (1. - x) / (SAMPLERATE as f32 / freq);
            }
            self.phase = self.phase.rem_euclid(1.);
            output[i] = self.phase;
        }
    }
}
