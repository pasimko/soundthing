use std::sync::mpsc::Sender;
use egui::Id;
use egui::{
    Color32, Pos2, Rect, Sense, Shape, Stroke, Painter,
    Widget as _, emath,
    pos2,
};

use wasm_bindgen::JsValue;
use web_sys::console;

use crate::nodes::{*, oscillators::*, adsr::AdsrNode, graph::AudioGraphMessage, output::OutputNode,
phasor::Phasor, phasor::PhaselessSineOsc};

pub struct Canvas {
    graph_handler: Sender<AudioGraphMessage>,
    node_parameters: Vec<NodeParameter>,
    sources: Vec<(usize, usize)>,
}

impl Canvas {
    /// Called once before the first frame.
    pub fn new(graph_handler: Sender<AudioGraphMessage>) -> Self {
        let (output, output_handler) = OutputNode::new();
        let _ = graph_handler.send(AudioGraphMessage::AddNode(Box::new(output)));
        Self {
            graph_handler,
            node_parameters: vec![NodeParameter::Output(output_handler)],
            sources: vec!(),
        }
    }
    pub fn render_nodes(&mut self, ctx: &egui::Context) {
        let mut results = vec!();
        for (idx, p) in self.node_parameters.iter_mut().enumerate() {
            match p {
                NodeParameter::Osc(p) => {
                    let result = egui::Window::new(p.name.clone()).id(Id::new(idx)).show(ctx, |ui| {
                        // TODO
                        // phase control node
                        // 
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                if ui.add(egui::Button::new("phase")).clicked() {
                                    // connection mode
                                }
                                if ui.add(egui::Button::new("phase")).clicked() {
                                    // connection mode
                                }
                            });
                            ui.vertical(|ui| {
                                let freq_res = ui.add(egui::Slider::new(&mut p.freq, 20.0..=2000.0).text("frequency").logarithmic(true));
                                let vol_res = ui.add(egui::Slider::new(&mut p.target_vol, 0..=100).text("volume"));
                                if freq_res.changed() {
                                    p.sender.send(Message::Frequency(p.freq)).unwrap();
                                }
                                if vol_res.changed() {
                                    p.sender.send(Message::Volume(p.target_vol)).unwrap();
                                }
                            });
                            ui.vertical(|ui| {
                                if ui.add(egui::Button::new("out")).clicked() {
                                    // connector mode
                                }
                            });
                        });
                    });
                    results.push(result);
                }
                NodeParameter::PhaselessOsc(p) => {
                    let result = egui::Window::new(p.name.clone()).id(Id::new(idx)).show(ctx, |ui| {
                        let vol_res = ui.add(egui::Slider::new(&mut p.target_vol, 0..=100).text("volume"));
                        if vol_res.changed() {
                            p.sender.send(Message::Volume(p.target_vol)).unwrap();
                        }
                    });
                    results.push(result);
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

                        // Draw phase bender graph
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

                    results.push(result);
                }
                NodeParameter::Adsr(p) => {
                    let result = egui::Window::new(p.name.clone()).id(Id::new(idx)).show(ctx, |ui| {
                        let on = ui.add(egui::Button::new("on"));
                        let on = on.hovered();
                        p.on = on;
                        p.sender.send(Message::On(on)).unwrap();
                    });
                    results.push(result);
                }
                NodeParameter::Output(p) => {
                    let result = egui::Window::new(p.name.clone()).id(Id::new(idx)).show(ctx, |ui| {
                    });
                    results.push(result);
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
        let painter = ctx.layer_painter(egui::LayerId::background());
        for (source_idx, sink_idx) in self.sources.iter() {
            if let Some(ir) = &results[*source_idx] {
                let (top_left, bot_right) = (ir.response.rect.min, ir.response.rect.max);
                if let Some(ir) = &results[*sink_idx] {
                    let parent_point = Pos2::new(bot_right.x, (top_left.y+bot_right.y) / 2.);
                    let (top_left, bot_right) = (ir.response.rect.min, ir.response.rect.max);
                    let child_point = Pos2::new(top_left.x, (top_left.y+bot_right.y) / 2.);
                    let line = Shape::line(vec![parent_point, child_point], Stroke::new(5.0, Color32::BLACK));
                    painter.add(line);
                }
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

            ui.horizontal(|ui| {
                ui.menu_button("Add oscillator", |ui| {
                    ui.set_width(100.0); // To make sure we wrap long text
                    if ui.button("Sine").clicked() {
                        let (new_osc, new_osc_handler) = SineOsc::new();

                        self.node_parameters.push(NodeParameter::Osc(new_osc_handler));
                        self.sources.push((self.node_parameters.len()-1, 0));

                        let _ = self.graph_handler.send(AudioGraphMessage::AddNode(Box::new(new_osc)));
                        let _ = self.graph_handler.send(AudioGraphMessage::AddEdge((self.node_parameters.len()-1, 0)));

                        ui.close();
                    }
                    if ui.button("Square").clicked() {
                        let (new_osc, new_osc_handler) = SquareOsc::new();

                        self.node_parameters.push(NodeParameter::Osc(new_osc_handler));
                        self.sources.push((self.node_parameters.len()-1, 0));

                        let _ = self.graph_handler.send(AudioGraphMessage::AddNode(Box::new(new_osc)));
                        let _ = self.graph_handler.send(AudioGraphMessage::AddEdge((self.node_parameters.len()-1, 0)));

                        ui.close();
                    }
                    if ui.button("Sawtooth").clicked() {
                        let (new_osc, new_osc_handler) = SawtoothOsc::new();

                        self.node_parameters.push(NodeParameter::Osc(new_osc_handler));
                        self.sources.push((self.node_parameters.len()-1, 0));

                        let _ = self.graph_handler.send(AudioGraphMessage::AddNode(Box::new(new_osc)));
                        let _ = self.graph_handler.send(AudioGraphMessage::AddEdge((self.node_parameters.len()-1, 0)));
                        ui.close();
                    }
                    if ui.button("Phaseless Sine").clicked() {
                        let (new_osc, new_phasor_handler) = PhaselessSineOsc::new();

                        self.node_parameters.push(NodeParameter::PhaselessOsc(new_phasor_handler));
                        self.sources.push((self.node_parameters.len()-1, 0));

                        let _ = self.graph_handler.send(AudioGraphMessage::AddNode(Box::new(new_osc)));
                        let _ = self.graph_handler.send(AudioGraphMessage::AddEdge((1, self.node_parameters.len()-1)));
                        let _ = self.graph_handler.send(AudioGraphMessage::AddEdge((self.node_parameters.len()-1, 0)));

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

            let painter = Painter::new(ctx.clone(), ui.layer_id(), ui.clip_rect());
            self.render_nodes(ctx);
        });
    }
}


// TODO TESTS
// 1. Make sure removing node removes all the edges
