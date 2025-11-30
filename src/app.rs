use std::sync::mpsc::Sender;
use egui::Id;
use egui::{
    Color32, Pos2, Rect, Sense, Shape, Stroke,
    Widget as _, emath,
    pos2,
};

use wasm_bindgen::JsValue;
use web_sys::console;

use crate::nodes::{*, oscillators::*, adsr::AdsrNode, graph::AudioGraphMessage};

pub struct Canvas {
    graph_handler: Sender<AudioGraphMessage>,
    node_parameters: Vec<NodeParameter>,
}

impl Canvas {
    /// Called once before the first frame.
    pub fn new(graph_handler: Sender<AudioGraphMessage>) -> Self {
        Self {
            graph_handler,
            node_parameters: vec!(),
        }
    }
    pub fn render_nodes(&mut self, ctx: &egui::Context) {
        for (idx, p) in self.node_parameters.iter_mut().enumerate() {
            match p {
                NodeParameter::Osc(p) => {
                    let result = egui::Window::new(p.name.clone()).id(Id::new(idx)).show(ctx, |ui| {
                        let freq_res = ui.add(egui::Slider::new(&mut p.freq, 20.0..=2000.0).text("frequency").logarithmic(true));
                        let vol_res = ui.add(egui::Slider::new(&mut p.target_vol, 0..=100).text("volume"));
                        if freq_res.changed() {
                            p.sender.send(Message::Frequency(p.freq)).unwrap();
                        }
                        if vol_res.changed() {
                            p.sender.send(Message::Volume(p.target_vol)).unwrap();
                        }
                        // let stroke = ui.style().interact(&point_response).fg_stroke;

                        let (response, painter) =
                            ui.allocate_painter(ui.available_size_before_wrap(), Sense::drag());
                        let circle = Shape::circle_stroke(Pos2::new(500., 500.), 100.0, Stroke::new(10.0, Color32::BLACK));
                        painter.add(circle);
                    });
                }
                NodeParameter::PhaselessOsc(p) => {
                    let result = egui::Window::new(p.name.clone()).id(Id::new(idx)).show(ctx, |ui| {
                        let vol_res = ui.add(egui::Slider::new(&mut p.target_vol, 0..=100).text("volume"));
                        if vol_res.changed() {
                            p.sender.send(Message::Volume(p.target_vol)).unwrap();
                        }
                    });
                }
                NodeParameter::Phasor(p) => {
                    let result = egui::Window::new(p.name.clone()).id(Id::new(idx)).show(ctx, |ui| {
                        ui.set_max_width(200.0);
                        ui.set_max_height(300.0);
                        let (mut response, painter) =
                            ui.allocate_painter(ui.available_size_before_wrap(), Sense::drag());

                        let to_screen = emath::RectTransform::from_to(
                            Rect::from_min_size(Pos2::ZERO, response.rect.square_proportions()),
                            response.rect,
                        );
                        let from_screen = to_screen.inverse();


                        if let Some(pointer_pos) = response.interact_pointer_pos() {
                            let canvas_pos = from_screen * pointer_pos;
                            p.point.0 = canvas_pos[0];
                            p.point.1 = canvas_pos[1];
                            p.sender.send(Message::Center(p.point)).unwrap();
                            response.mark_changed();
                            console::log_2(&JsValue::from_f64(p.point.0 as f64), &JsValue::from_f64(p.point.1 as f64));
                        }

                        let lines = [vec![pos2(0., 1.), pos2(p.point.0, p.point.1), pos2(1., 0.)]];

                        let shapes = lines
                            .iter()
                            .filter(|line| line.len() >= 2)
                            .map(|line| {
                                let points: Vec<Pos2> = line.iter().map(|p| to_screen * *p).collect();
                                egui::Shape::line(points, Stroke::new(2.0, Color32::BLACK))
                            });

                        painter.extend(shapes);
                        let freq_res = ui.add(egui::Slider::new(&mut p.freq, 20.0..=2000.0).text("frequency").logarithmic(true));
                        if freq_res.changed() {
                            p.sender.send(Message::Frequency(p.freq)).unwrap();
                        }
                    });
                }
                NodeParameter::Adsr(p) => {
                    let result = egui::Window::new(p.name.clone()).id(Id::new(idx)).show(ctx, |ui| {
                        let on = ui.add(egui::Button::new("on"));
                        let on = on.hovered();
                        p.on = on;
                        p.sender.send(Message::On(on)).unwrap();
                    });
                }
                // Parameter::Timer(p) => {
                //     egui::Window::new(&p.name).id(Id::new(id)).show(ctx, |ui| {
                //         // let on = ui.add(egui::Button::new("on"));
                //         // let on = on.hovered();
                //         // p.on = on;
                //         // p.sender.send(AdsrMessage::On(on)).unwrap();
                //     });
                // }
            }
        }
    }
}

impl eframe::App for Canvas {
    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                // NOTE: no File->Quit on web pages!
                let is_web = cfg!(target_arch = "wasm32");
                if !is_web {
                    ui.menu_button("File", |ui| {
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    ui.add_space(16.0);
                }


                egui::widgets::global_theme_preference_buttons(ui);
            });
        });

        // TODO make this an egui::Scene
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("music");

            // ui.set_max_width(200.0); // To make sure we wrap long text

            ui.horizontal(|ui| {
                ui.menu_button("Add oscillator", |ui| {
                    ui.set_width(100.0); // To make sure we wrap long text
                    if ui.button("Sine").clicked() {
                        let (new_osc, new_osc_handler) = SineOsc::new();
                        let _ = self.graph_handler.send(AudioGraphMessage::AddNode(Box::new(new_osc)));
                        self.node_parameters.push(NodeParameter::Osc(new_osc_handler));
                        let _ = self.graph_handler.send(AudioGraphMessage::AddEdge((self.node_parameters.len(), 0)));
                        ui.close();
                    }
                    if ui.button("Square").clicked() {
                        let (new_osc, new_osc_handler) = SquareOsc::new();
                        let _ = self.graph_handler.send(AudioGraphMessage::AddNode(Box::new(new_osc)));
                        self.node_parameters.push(NodeParameter::Osc(new_osc_handler));
                        let _ = self.graph_handler.send(AudioGraphMessage::AddEdge((self.node_parameters.len(), 0)));
                        ui.close();
                    }
                    if ui.button("Sawtooth").clicked() {
                        let (new_osc, new_osc_handler) = SawtoothOsc::new();
                        let _ = self.graph_handler.send(AudioGraphMessage::AddNode(Box::new(new_osc)));
                        self.node_parameters.push(NodeParameter::Osc(new_osc_handler));
                        let _ = self.graph_handler.send(AudioGraphMessage::AddEdge((self.node_parameters.len(), 1)));
                        ui.close();
                    }
                    if ui.button("Phaseless Sine").clicked() {
                        let (new_osc, new_phasor_handler) = PhaselessSineOsc::new();
                        let _ = self.graph_handler.send(AudioGraphMessage::AddNode(Box::new(new_osc)));
                        self.node_parameters.push(NodeParameter::PhaselessOsc(new_phasor_handler));
                        let _ = self.graph_handler.send(AudioGraphMessage::AddEdge((1, self.node_parameters.len())));
                        let _ = self.graph_handler.send(AudioGraphMessage::AddEdge((self.node_parameters.len(), 0)));
                        ui.close();
                    }
                });
                ui.menu_button("Add controller", |ui| {
                    ui.set_width(100.0); // To make sure we wrap long text
                    if ui.button("Phasor").clicked() {
                        let (new_osc, new_phasor_handler) = Phasor::new();
                        let _ = self.graph_handler.send(AudioGraphMessage::AddNode(Box::new(new_osc)));
                        self.node_parameters.push(NodeParameter::Phasor(new_phasor_handler));
                        ui.close();
                    }
                    if ui.button("ADSR").clicked() {
                        let (new_osc, new_phasor_handler) = AdsrNode::new();
                        let _ = self.graph_handler.send(AudioGraphMessage::AddNode(Box::new(new_osc)));
                        self.node_parameters.push(NodeParameter::Adsr(new_phasor_handler));
                        let _ = self.graph_handler.send(AudioGraphMessage::AddEdge((self.node_parameters.len(), 0)));
                        ui.close();
                    }
                });
            });

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                egui::warn_if_debug_build(ui);
            });

            self.render_nodes(ctx);
        });
    }
}
    // TODO TEMPORARY 
//    if let Some(r) = result {
//        if r.response.contains_pointer() {
//            ctx.input(|i| {
//                for &key in i.keys_down.iter() {
//                    if i.key_pressed(key) {
//                        p.freq = match key {
//                            egui::Key::A => 220.,
//                            egui::Key::S => 220.+110.*1.,
//                            egui::Key::D => 220.+110.*2.,
//                            egui::Key::F => 220.+110.*3.,
//                            egui::Key::G => 220.+110.*4.,
//                            egui::Key::H => 220.+110.*5.,
//                            egui::Key::J => 220.+220.*6.,
//                            egui::Key::K => 220.+220.*7.,
//                            egui::Key::L => 220.+220.*8.,
//                            _ => 110.,
//                        };
//                        p.sender.send(Message::Frequency(p.freq)).unwrap();
//                    }
//                }
//            });
//        }
//    }
//}
