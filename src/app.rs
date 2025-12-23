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
math::MathNode, PortResponses,
phasor::PhaseBender};

enum Mode {
    Normal,
    SelectSink(graph::NodeId, graph::PortId),
    SelectSource(graph::NodeId, graph::PortId),
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
    // TODO make this not 1000 lines long
    pub fn render_nodes(&mut self, ctx: &egui::Context) {
        let mut responses: Vec<PortResponses> = vec!();
        for (idx, p) in self.node_parameters.iter_mut().enumerate() {
            match p {
                NodeParameter::Osc(p) => responses.push(p.draw(ctx, idx)),
                NodeParameter::Phasor(p) => responses.push(p.draw(ctx, idx)),
                // TODO update with new edge handling
                NodeParameter::Adsr(p) => {
                    egui::Window::new(p.name.clone()).id(Id::new(idx)).show(ctx, |ui| {
                        let on = ui.add(egui::Button::new("on"));
                        let on = on.hovered();
                        p.on = on;
                        p.sender.send(Message::On(on)).unwrap();
                    });
                    // results.push(result);
                }
                NodeParameter::Math(p) => responses.push(p.draw(ctx, idx)),
                NodeParameter::Output(p) => responses.push(p.draw(ctx, idx)),
            }
        }
        // Handle drawing all the edges
        for (idx, node) in responses.iter().enumerate() {
            for (jdx, port) in node.inputs.iter().enumerate() {
                if port.clicked() {
                    let current_node = (graph::NodeId(idx), graph::PortId(jdx));
                    match self.current_mode {
                        // Finish creating new edge
                        Mode::SelectSink(node_id, port) => {
                            let new_edge = graph::Edge { 
                                from: (node_id, port), 
                                to: current_node,
                            };
                            self.incoming_edges.push(new_edge.clone());
                            let _ = self.graph_handler.send(AudioGraphMessage::AddEdge(new_edge));
                            self.current_mode = Mode::Normal;
                        },
                        // Start creating new edge
                        Mode::Normal => {
                            self.current_mode = Mode::SelectSource(current_node.0, current_node.1);
                        },
                        _ => {}
                    }
                }
            }
            for (jdx, port) in node.outputs.iter().enumerate() {
                if port.clicked() {
                    let current_node = (graph::NodeId(idx), graph::PortId(jdx));
                    match self.current_mode {
                        // Finish creating new edge
                        Mode::SelectSource(node_id, port) => {
                            let new_edge = graph::Edge { 
                                from: current_node,
                                to: (node_id, port), 
                            };
                            self.incoming_edges.push(new_edge.clone());
                            let _ = self.graph_handler.send(AudioGraphMessage::AddEdge(new_edge));
                            self.current_mode = Mode::Normal;
                        },
                        // Start creating new edge
                        Mode::Normal => {
                            self.current_mode = Mode::SelectSink(current_node.0, current_node.1);
                        },
                        _ => {}
                    }
                }
            }
        }

        // Draw all the edges
        // TODO this breaks if a widget isn't drawn -- collapsed window, maybe other cases
        // Switching to an ID system would help, probably
        let painter = ctx.layer_painter(egui::LayerId::background());
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
        // Draw the edge currently being created
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
                        let (new_osc, new_osc_handler) = OscNode::new();
                        self.node_parameters.push(NodeParameter::Osc(new_osc_handler));
                        let _ = self.graph_handler.send(AudioGraphMessage::AddNode(Box::new(new_osc)));
                        ui.close();
                    }
                });

                ui.menu_button("Numeric", |ui| {
                    ui.set_width(100.0); // To make sure we wrap long text
                    if ui.button("Math").clicked() {
                        let (new_node, node_handler) = MathNode::new();
                        self.node_parameters.push(NodeParameter::Math(node_handler));
                        let _ = self.graph_handler.send(AudioGraphMessage::AddNode(Box::new(new_node)));
                        ui.close();
                    }
                });
                ui.menu_button("Add controller", |ui| {
                    ui.set_width(100.0); // To make sure we wrap long text
                    if ui.button("Phasor").clicked() {
                        let (new_osc, new_phasor_handler) = PhaseBender::new();
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
        // TODO hotkeys here?
        if ctx.input(|i|
            i.pointer.any_click()) 
            && panel_response.response.contains_pointer() 
            && !ctx.is_using_pointer() 
        {
            self.current_mode = Mode::Normal;
        }
    }
}
