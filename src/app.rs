use std::sync::mpsc::Sender;
use super::graph;
use egui::Id;
use egui::{
    Color32, Pos2, Rect, Sense, Shape, Stroke,
    emath,
    pos2,
};

use wasm_bindgen::JsValue;
use web_sys::console;

use crate::nodes::{*, oscillators::*, adsr::AdsrNode, graph::AudioGraphMessage, output::OutputNode,
phasor::Phasor, phasor::PhaselessSineOsc};

enum Mode {
    Normal,
    SelectSink(graph::NodeId, graph::PortId),
    SelectSource(graph::NodeId, graph::PortId),
}

struct PortResponses {
    inputs: Vec<egui::Response>,
    outputs: Vec<egui::Response>,
}

impl PortResponses {
    fn new() -> Self {
        Self {
            inputs: vec!(),
            outputs: vec!(),
        }
    }
}

pub struct Canvas {
    graph_handler: Sender<AudioGraphMessage>,
    node_parameters: Vec<NodeParameter>,
    incoming_edges: Vec<graph::Edge>,
    current_mode: Mode,
}

impl Canvas {
    /// Called once before the first frame.
    pub fn new(graph_handler: Sender<AudioGraphMessage>) -> Self {
        let (output, output_handler) = OutputNode::new();
        let _ = graph_handler.send(AudioGraphMessage::AddNode(Box::new(output)));
        Self {
            graph_handler,
            node_parameters: vec![NodeParameter::Output(output_handler)],
            incoming_edges: vec!(),
            current_mode: Mode::Normal,
        }
    }
    pub fn render_nodes(&mut self, ctx: &egui::Context) {
        let mut responses: Vec<PortResponses> = vec!();
        for (idx, p) in self.node_parameters.iter_mut().enumerate() {
            let mut port_responses = PortResponses::new();
            match p {
                NodeParameter::Osc(p) => {
                    let label = match p.waveform {
                        Waveform::Saw => "Sawtooth Oscillator",
                        Waveform::Sine => "Sine Oscillator",
                        Waveform::Square => "Square Oscillator",
                        Waveform::Tri => "Triangle Oscillator",
                    };
                    egui::Window::new(label).id(Id::new(idx)).show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            // input ports
                            ui.vertical(|ui| {
                                let frequency_button_response = ui.add(egui::Button::new("frequency"));
                                if frequency_button_response.clicked() {
                                    match self.current_mode {
                                        // Finish creating new edge
                                        Mode::SelectSink(node_id, port) => {
                                            let new_edge = graph::Edge { 
                                                from: (node_id, port), 
                                                to: (graph::NodeId(idx), graph::PortId(0)),
                                            };
                                            self.incoming_edges.push(new_edge.clone());
                                            let _ = self.graph_handler.send(AudioGraphMessage::AddEdge(new_edge));
                                            self.current_mode = Mode::Normal;
                                        },
                                        // Start creating new edge
                                        Mode::Normal => {
                                            self.current_mode = Mode::SelectSource(graph::NodeId(idx), graph::PortId(0));
                                        },
                                        _ => {}
                                    }
                                }
                                port_responses.inputs.push(frequency_button_response);

                                let volume_button_response = ui.add(egui::Button::new("volume"));
                                if volume_button_response.clicked() {
                                    match self.current_mode {
                                        // Finish creating new edge
                                        Mode::SelectSink(node_id, port) => {
                                            let new_edge = graph::Edge { 
                                                from: (node_id, port), 
                                                to: (graph::NodeId(idx), graph::PortId(1)),
                                            };
                                            self.incoming_edges.push(new_edge.clone());
                                            let _ = self.graph_handler.send(AudioGraphMessage::AddEdge(new_edge));
                                            self.current_mode = Mode::Normal;
                                        },
                                        // Start creating new edge
                                        Mode::Normal => {
                                            self.current_mode = Mode::SelectSource(graph::NodeId(idx), graph::PortId(0));
                                        },
                                        _ => {}
                                    }
                                }
                                port_responses.inputs.push(volume_button_response);
                                if ui.add(egui::Button::new("phase")).clicked() {
                                    // connection mode
                                }
                            });
                            // sliders, other non-port UI stuff
                            ui.vertical(|ui| {
                                let freq_res = ui.add_enabled(true,
                                    egui::Slider::new(&mut p.freq, 0.0..=2000.0)
                                        .text("frequency")
                                        .logarithmic(true));
                                let vol_res = ui.add_enabled(true,
                                    egui::Slider::new(&mut p.target_vol, 0..=100)
                                        .text("volume"));
                                if freq_res.changed() {
                                    let _ = p.sender.send(Message::Frequency(p.freq));
                                }
                                if vol_res.changed() {
                                    let _ = p.sender.send(Message::Volume(p.target_vol));
                                }
                                ui.menu_button("waveform", |ui| {
                                    ui.set_width(100.0); // To make sure we wrap long text
                                    if ui.button("Sine").clicked() {
                                        p.waveform = Waveform::Sine;
                                        let _ = p.sender.send(Message::Waveform(oscillators::Waveform::Sine));
                                    }
                                    if ui.button("Square").clicked() {
                                        p.waveform = Waveform::Square;
                                        let _ = p.sender.send(Message::Waveform(oscillators::Waveform::Square));
                                    }
                                    if ui.button("Saw").clicked() {
                                        p.waveform = Waveform::Saw;
                                        let _ = p.sender.send(Message::Waveform(oscillators::Waveform::Saw));
                                    }
                                });
                            });
                            // output ports
                            ui.vertical(|ui| {
                                let out_button_response = ui.add(egui::Button::new("out"));
                                if out_button_response.clicked() {
                                    self.current_mode = Mode::SelectSink(graph::NodeId(idx), graph::PortId(0));
                                }
                                port_responses.outputs.push(out_button_response);
                            });
                        });
                    });
                    responses.push(port_responses);
                }
                NodeParameter::PhaselessOsc(p) => {
                    egui::Window::new(p.name.clone()).id(Id::new(idx)).show(ctx, |ui| {
                        let vol_res = ui.add(egui::Slider::new(&mut p.target_vol, 0..=100).text("volume"));
                        if vol_res.changed() {
                            p.sender.send(Message::Volume(p.target_vol)).unwrap();
                        }
                    });
                }
                NodeParameter::Phasor(p) => {
                    egui::Window::new(p.name.clone()).id(Id::new(idx)).show(ctx, |ui| {
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

                    // results.push(result);
                }
                NodeParameter::Adsr(p) => {
                    egui::Window::new(p.name.clone()).id(Id::new(idx)).show(ctx, |ui| {
                        let on = ui.add(egui::Button::new("on"));
                        let on = on.hovered();
                        p.on = on;
                        p.sender.send(Message::On(on)).unwrap();
                    });
                    // results.push(result);
                }
                NodeParameter::Output(p) => {
                    egui::Window::new(p.name.clone()).id(Id::new(idx)).show(ctx, |ui| {
                        ui.vertical(|ui| {
                            let in_button_response = ui.add(egui::Button::new("in"));
                            if in_button_response.clicked() {
                                match self.current_mode {
                                    Mode::SelectSink(node_id, port) => {
                                        let new_edge = graph::Edge { 
                                            from: (node_id, port), 
                                            to: (graph::NodeId(idx), graph::PortId(0)),
                                        };

                                        self.incoming_edges.push(new_edge.clone());
                                        let _ = self.graph_handler.send(AudioGraphMessage::AddEdge(new_edge));
                                        console::log_2(&JsValue::from_f64(node_id.0 as f64), &JsValue::from_f64(idx as f64));
                                        self.current_mode = Mode::Normal;
                                    },
                                    Mode::Normal => {

                                    }, // switch to selectSource
                                    _ => {}
                                }
                            }
                            port_responses.inputs.push(in_button_response);
                        });
                    });
                    responses.push(port_responses);
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
        // Draw all the edges
        // TODO this breaks if a widget isn't drawn -- collapsed window, maybe other cases
        for edge in self.incoming_edges.iter() {
            let source_idx = edge.from.0.0;
            let sink_idx = edge.to.0.0;
            let source_port_response = &responses[source_idx].outputs[edge.from.1.0 as usize];
            let sink_port_response = &responses[sink_idx].inputs[edge.to.1.0 as usize];

            // source port coords
            let (top_left, bot_right) = (source_port_response.rect.min, source_port_response.rect.max);
            let parent_point = Pos2::new(bot_right.x, (top_left.y+bot_right.y) / 2.);

            // sink port coords
            let (top_left, bot_right) = (sink_port_response.rect.min, sink_port_response.rect.max);
            let child_point = Pos2::new(top_left.x, (top_left.y+bot_right.y) / 2.);

            let line = Shape::line(vec![parent_point, child_point], Stroke::new(2.0, Color32::GRAY));
            painter.add(line);
        }
        let painter = ctx.layer_painter(egui::LayerId::new(egui::layers::Order::Foreground, Id::new("ephemeral interaction")));
        match self.current_mode {
            Mode::SelectSource(node_id, port) => {
                let sink_port_response = &responses[node_id.0].inputs[port.0 as usize];

                // source port coords
                let (top_left, bot_right) = (sink_port_response.rect.min, sink_port_response.rect.max);
                let parent_point = Pos2::new(top_left.x, (top_left.y+bot_right.y) / 2.);
                if let Some(mouse_pos) = ctx.input(|i| i.pointer.latest_pos()) {
                    let line = Shape::line(vec![parent_point, mouse_pos], Stroke::new(2.0, Color32::PURPLE));
                    painter.add(line);
                }
            }
            Mode::SelectSink(node_id, port) => {
                let source_port_response = &responses[node_id.0]
                    .outputs[port.0 as usize];

                // source port coords
                let (top_left, bot_right) = (source_port_response.rect.min, source_port_response.rect.max);
                let parent_point = Pos2::new(bot_right.x, (top_left.y+bot_right.y) / 2.);
                if let Some(mouse_pos) = ctx.input(|i| i.pointer.latest_pos()) {
                    let line = Shape::line(vec![parent_point, mouse_pos], Stroke::new(2.0, Color32::PURPLE));
                    painter.add(line);
                }
            }
            _ => {}
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
        let panel_response = egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("music");

            ui.horizontal(|ui| {
                ui.menu_button("Add oscillator", |ui| {
                    ui.set_width(100.0); // To make sure we wrap long text
                    if ui.button("Sine").clicked() {
                        let (new_osc, new_osc_handler) = Osc::new();
                        self.node_parameters.push(NodeParameter::Osc(new_osc_handler));
                        let _ = self.graph_handler.send(AudioGraphMessage::AddNode(Box::new(new_osc)));
                        ui.close();
                    }
                    if ui.button("Phaseless Sine").clicked() {
                        let (new_osc, new_phasor_handler) = PhaselessSineOsc::new();
                        self.node_parameters.push(NodeParameter::PhaselessOsc(new_phasor_handler));
                        let _ = self.graph_handler.send(AudioGraphMessage::AddNode(Box::new(new_osc)));
                        ui.close();
                    }
                });

                ui.menu_button("Add controller", |ui| {
                    ui.set_width(100.0); // To make sure we wrap long text
                    if ui.button("Phasor").clicked() {
                        let (new_osc, new_phasor_handler) = Phasor::new();
                        self.node_parameters.push(NodeParameter::Phasor(new_phasor_handler));
                        let _ = self.graph_handler.send(AudioGraphMessage::AddNode(Box::new(new_osc)));
                        ui.close();
                    }
                    if ui.button("ADSR").clicked() {
                        let (new_osc, new_phasor_handler) = AdsrNode::new();
                        self.node_parameters.push(NodeParameter::Adsr(new_phasor_handler));
                        let _ = self.graph_handler.send(AudioGraphMessage::AddNode(Box::new(new_osc)));
                        ui.close();
                    }
                });
            });

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                egui::warn_if_debug_build(ui);
            });

            self.render_nodes(ctx);
        });
        // TODO also maybe make drags delete edges?
        if ctx.input(|i|
            i.pointer.any_click()) 
            && panel_response.response.contains_pointer() 
            && !ctx.is_using_pointer() 
        {
            web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("what"));
            self.current_mode = Mode::Normal;
        }
    }
}


// TODO TESTS
// 1. Make sure removing node removes all the edges
