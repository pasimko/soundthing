use std::sync::mpsc::Sender;

use super::{PortInfo, Node, SAMPLERATE, NodeUi, PortDescriptions, drain_messages};
use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;


#[derive(Debug, Clone)]
pub enum Waveform {
    Sine,
    Square,
    Tri,
    Saw,
}
use super::graph;

use egui::{
    Color32, Pos2, Rect, Sense, Stroke,
    emath,
    pos2,
};

pub enum PhasorMessage {
    SetA(f32),
    SetB(f32),
    Frequency(f32),
}

#[derive(Debug, Clone)]
pub struct PhasorParameters {
    pub freq: f32,
    pub point: (f32, f32),
    pub sender: Sender<PhasorMessage>,
    pub name: String,
}

impl PhasorParameters {
    fn handle_message(&mut self, msg: PhasorMessage) {
        match msg {
            PhasorMessage::SetA(val) => self.point.0 = val,
            PhasorMessage::SetB(val) => self.point.1 = val,
            PhasorMessage::Frequency(val) => self.freq = val,
        }
    }
}

impl NodeUi for PhasorParameters {
    fn duplicate(&self) -> Option<(Box<dyn Node>, Box<dyn NodeUi>)> {
        let (node, params) = PhaseBender::from_params(self.clone());
        Some((Box::new(node), Box::new(params)))
    }

    fn title(&self) -> String {
        "Phase Bender".to_string()
    }

    fn ports(&self) -> PortDescriptions {
        PortDescriptions::with_ports(
            vec![
                PortInfo::input("x", "Horizontal position of the bend point, from 0 to 1."),
                PortInfo::input("y", "Vertical position of the bend point, from 0 to 1."),
                PortInfo::input("frequency", "Cycles per second. Overrides the slider."),
            ],
            vec![
                PortInfo::output("phase", "A 0 to 1 ramp shaped by the bend point. Feed it to an oscillator's phase."),
            ]
        )
    }

    fn draw(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
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
                    self.sender.send(PhasorMessage::SetA(self.point.0)).unwrap();
                    self.sender.send(PhasorMessage::SetB(self.point.1)).unwrap();
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
                let freq_res = ui.add(egui::Slider::new(&mut self.freq, 0.001..=2000.0).text("frequency").logarithmic(true));
                if freq_res.changed() {
                    self.sender.send(PhasorMessage::Frequency(self.freq)).unwrap();
                }
            });
        });
    }
}

pub struct PhaseBender {
    params: PhasorParameters,
    phase: f32,
    freq: f32,
    msg_receiver: Receiver<PhasorMessage>,
    pub point: (f32, f32),
}

impl PhaseBender {
    pub fn new() -> (Self, PhasorParameters) {
        let (msg_sender, msg_receiver) = channel();
        let params = PhasorParameters {
            freq: 440.,
            sender: msg_sender,
            name: "Phasor".to_string(),
            point: (0.5, 0.5),
        };
        Self::build(params, msg_receiver)
    }

    pub fn from_params(mut params: PhasorParameters) -> (Self, PhasorParameters) {
        let (msg_sender, msg_receiver) = channel();
        params.sender = msg_sender;
        Self::build(params, msg_receiver)
    }

    fn build(params: PhasorParameters, msg_receiver: Receiver<PhasorMessage>) -> (Self, PhasorParameters) {
        let point = params.point;
        let freq = params.freq;
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
        drain_messages(&self.msg_receiver, |msg| self.params.handle_message(msg));
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
                .unwrap_or(1. - self.params.point.1);
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
