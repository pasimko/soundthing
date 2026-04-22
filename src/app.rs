use std::sync::mpsc::Sender;
use super::graph;
use egui::Id;
use egui::{
    Color32, Pos2, Shape, Stroke,
};


use crate::nodes::{*, oscillators::*, adsr::AdsrNode, graph::AudioGraphMessage, output::OutputNode,
math::MathNode, PortDescriptions, metronome::MetronomeNode, sequencer::SequencerNode,
phasor::PhaseBender, delay::DelayNode};

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
    scene_rect: egui::Rect,
}

pub struct PortPositions {
    inputs: Vec<egui::Pos2>,
    outputs: Vec<egui::Pos2>,
}

impl PortPositions {
    fn new() -> Self {
        PortPositions {
            inputs: Vec::new(),
            outputs: Vec::new()
        }
    }
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
            scene_rect: egui::Rect::ZERO,
        }
    }
    // TODO make this not 1000 lines long
    pub fn render_nodes(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let mut port_positions = Vec::new();
        for (idx, p) in self.node_parameters.iter_mut().enumerate() {
            port_positions.push(PortPositions::new());
            let mut port_info : Option<(PortDescriptions, egui::Response)> = None;
            match p {
                NodeParameter::Osc(p) => port_info = Some(p.draw(ctx, idx)),
                NodeParameter::Phasor(p) => port_info = Some(p.draw(ctx, idx)),
                NodeParameter::Adsr(p) => port_info = Some(p.draw(ctx, idx)),
                NodeParameter::Math(p) => port_info = Some(p.draw(ctx, idx)),
                NodeParameter::Output(p) => port_info = Some(p.draw(ctx, idx)),
                NodeParameter::Metronome(p) => port_info = Some(p.draw(ctx, idx)),
                NodeParameter::Sequencer(p) => port_info = Some(p.draw(ctx, idx)),
                NodeParameter::Delay(p) => port_info = Some(p.draw(ctx, idx)),
                _ => {}
            }

            // Draw ports
            let port_info = port_info.unwrap();
            let port_descriptions = port_info.0;
            let node_window_response = port_info.1;
            let painter = ctx.layer_painter(node_window_response.layer_id);

            // inputs
            for (port_idx, port_label) in port_descriptions.inputs.iter().enumerate() {
                let mut port_size = 5.5;
                let mut input_pos = node_window_response.rect.left_top();
                input_pos.x -= 10.;
                input_pos.y += port_size * 2. + 18. * port_idx as f32;
                port_positions[idx].inputs.push(input_pos);
                // sense clicks on port positions
                let button_rect = egui::Rect::from_pos(input_pos).expand(8.0);
                let port_id = node_window_response.id.with((port_idx + 1) * 100); // stupid. just preventing overlap
                let port_response = ui.interact(button_rect, port_id, egui::Sense::click());
                if port_response.hovered() {
                    port_size = 7.0;
                }
                painter.circle(input_pos, port_size, egui::Color32::BLACK, egui::Stroke::NONE);
                if port_response.clicked() {
                    let current_node = (graph::NodeId(idx), graph::PortId(port_idx));
                    match self.current_mode {
                        // Finish creating new edge
                        Mode::SelectSink(node_id, port) => {
                            let new_edge = graph::Edge { 
                                from: (node_id, port), 
                                to: current_node,
                            };
                            self.incoming_edges.push(new_edge);
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

            // outputs
            for (port_idx, port_label) in port_descriptions.outputs.iter().enumerate() {
                let mut port_size = 5.5;
                let mut port_pos = node_window_response.rect.right_top();
                port_pos.x += 10.;
                port_pos.y += port_size * 2. + 18. * port_idx as f32;
                port_positions[idx].outputs.push(port_pos);
                // sense clicks on port positions
                let button_rect = egui::Rect::from_pos(port_pos).expand(8.0);
                let port_id = node_window_response.id.with(port_idx);
                let port_response = ui.interact(button_rect, port_id, egui::Sense::click());
                if port_response.hovered() {
                    port_size = 7.0;
                }
                painter.circle(port_pos, port_size, egui::Color32::BLACK, egui::Stroke::NONE);
                if port_response.clicked() {
                    let current_node = (graph::NodeId(idx), graph::PortId(port_idx));
                    match self.current_mode {
                        // Finish creating new edge
                        Mode::SelectSource(node_id, port) => {
                            let new_edge = graph::Edge { 
                                from: current_node,
                                to: (node_id, port), 
                            };
                            self.incoming_edges.push(new_edge);
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
        // Draw edges
        let painter = ctx.layer_painter(egui::LayerId::background());
        for edge in self.incoming_edges.iter() {
            let source_idx = edge.from.0.0;
            let sink_idx = edge.to.0.0;
            let source_port_pos = port_positions[source_idx].outputs[edge.from.1.0];
            let sink_port_pos = port_positions[sink_idx].inputs[edge.to.1.0];

            painter.line(vec![source_port_pos, sink_port_pos], Stroke::new(2.0, Color32::GRAY));
        }

        // Draw the edge currently being created in a special color
        let painter = ctx.layer_painter(egui::LayerId::new(egui::layers::Order::Foreground, Id::new("ephemeral interaction")));
        match self.current_mode {
            Mode::SelectSource(node_id, port) => {
                let sink_port_pos = port_positions[node_id.0].inputs[port.0];

                // draw the line
                if let Some(mouse_pos) = ctx.input(|i| i.pointer.latest_pos()) {
                    painter.line(vec![sink_port_pos, mouse_pos], Stroke::new(2.0, Color32::PURPLE));
                }
            }
            Mode::SelectSink(node_id, port) => {
                let source_port_pos = port_positions[node_id.0].outputs[port.0];

                // draw the line
                if let Some(mouse_pos) = ctx.input(|i| i.pointer.latest_pos()) {
                    painter.line(vec![source_port_pos, mouse_pos], Stroke::new(2.0, Color32::PURPLE));
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

        let panel_response = egui::CentralPanel::default().show(ctx, |ui| {
            // TODO hotkeys and stuff here probably
            let bg_id = ui.id().with("bg_click");
            let bg_response = ui.interact(ui.max_rect(), bg_id, egui::Sense::click());
            if bg_response.clicked() {
                self.current_mode = Mode::Normal;
            }

            ui.heading("infinite recess");

            ui.horizontal(|ui| {
                ui.menu_button("add oscillator", |ui| {
                    ui.set_width(100.0); // To make sure we wrap long text
                    if ui.button("sine").clicked() {
                        let (new_osc, new_osc_handler) = OscNode::new();
                        self.node_parameters.push(NodeParameter::Osc(new_osc_handler));
                        let _ = self.graph_handler.send(AudioGraphMessage::AddNode(Box::new(new_osc)));
                        ui.close();
                    }
                });

                ui.menu_button("numeric", |ui| {
                    ui.set_width(100.0); // To make sure we wrap long text
                    if ui.button("math").clicked() {
                        let (new_node, node_handler) = MathNode::new();
                        self.node_parameters.push(NodeParameter::Math(node_handler));
                        let _ = self.graph_handler.send(AudioGraphMessage::AddNode(Box::new(new_node)));
                        ui.close();
                    }
                });
                ui.menu_button("add controller", |ui| {
                    ui.set_width(100.0); // To make sure we wrap long text
                    if ui.button("phasor").clicked() {
                        let (new_osc, new_phasor_handler) = PhaseBender::new();
                        self.node_parameters.push(NodeParameter::Phasor(new_phasor_handler));
                        let _ = self.graph_handler.send(AudioGraphMessage::AddNode(Box::new(new_osc)));
                        ui.close();
                    }
                    if ui.button("metronome").clicked() {
                        let (new_osc, new_metronome_handler) = MetronomeNode::new();
                        self.node_parameters.push(NodeParameter::Metronome(new_metronome_handler));
                        let _ = self.graph_handler.send(AudioGraphMessage::AddNode(Box::new(new_osc)));
                        ui.close();
                    }
                    if ui.button("envelope").clicked() {
                        let (new_osc, new_phasor_handler) = AdsrNode::new();
                        self.node_parameters.push(NodeParameter::Adsr(new_phasor_handler));
                        let _ = self.graph_handler.send(AudioGraphMessage::AddNode(Box::new(new_osc)));
                        ui.close();
                    }
                    if ui.button("sequencer").clicked() {
                        let (new_osc, new_sequencer_handler) = SequencerNode::new();
                        self.node_parameters.push(NodeParameter::Sequencer(new_sequencer_handler));
                        let _ = self.graph_handler.send(AudioGraphMessage::AddNode(Box::new(new_osc)));
                        ui.close();
                    }
                });
                ui.menu_button("effects", |ui| {
                    if ui.button("delay").clicked() {
                        let (new_delay, new_delay_handler) = DelayNode::new();
                        self.node_parameters.push(NodeParameter::Delay(new_delay_handler));
                        let _ = self.graph_handler.send(AudioGraphMessage::AddNode(Box::new(new_delay)));
                        ui.close();
                    }
                });
            });

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                egui::warn_if_debug_build(ui);
            });


            // TODO pan/zoom with
            // https://docs.rs/egui/latest/egui/struct.Context.html#method.set_transform_layer
            self.render_nodes(ctx, ui);
        });
    }
}
