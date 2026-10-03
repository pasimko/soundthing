use std::sync::mpsc::Sender;
use super::graph;
use egui::Id;
use egui::{
    Color32, Pos2, Stroke, Vec2, pos2,
    emath::TSTransform,
};


use crate::nodes::{*, oscillators::*, adsr::AdsrNode, graph::AudioGraphMessage, output::OutputNode,
math::MathNode, metronome::MetronomeNode, sequencer::SequencerNode, noise::NoiseNode,
phasor::PhaseBender, delay::DelayNode, reverb::ReverbNode};

enum Mode {
    Normal,
    ClickingSink(graph::NodeId, graph::PortId),
    ClickingSource(graph::NodeId, graph::PortId),
    EdgeKnife(Pos2),
    Cloning(CloneDrag)
}

pub struct Canvas {
    graph_handler: Sender<AudioGraphMessage>,
    node_parameters: Vec<Box<dyn NodeUi>>,
    // where each node's window goes; parallel to `node_parameters`
    placements: Vec<Option<Placement>>,
    incoming_edges: Vec<graph::Edge>,
    current_mode: Mode, // TODO maybe make this match? Maybe Option<Mode>?
                        // Clone drag, port drag, knife drag could all be different
                        // mode options
    // offset applied to every node window, in screen points
    pan: Vec2,
    // since knife drags are finished outside of the edge drawing loop, this saves the cut for
    // when edge positions are known
    pending_cut: Option<(Pos2, Pos2)>,
}

/// An alt-drag in progress. egui ties the drag to the original window, so instead the
/// original is pinned in place while the clone's window is moved by hand to follow the pointer.
#[derive(Clone, Copy)]
struct CloneDrag {
    original: usize,
    clone: usize,
    original_pos: Pos2,
    grab_offset: Vec2,
}

fn ccw(a: Pos2, b: Pos2, c: Pos2) -> bool {
    (c.y - a.y) * (b.x - a.x) > (b.y - a.y) * (c.x - a.x)
}

fn segments_intersect(a: Pos2, b: Pos2, c: Pos2, d: Pos2) -> bool {
    ccw(a, c, d) != ccw(b, c, d) && ccw(a, b, c) != ccw(a, b, d)
}

impl Canvas {
    /// Called once before the first frame.
    pub fn new(graph_handler: Sender<AudioGraphMessage>) -> Self {
        let (output, output_handler) = OutputNode::new();
        let _ = graph_handler.send(AudioGraphMessage::AddNode(Box::new(output)));
        Self {
            graph_handler,
            node_parameters: vec![Box::new(output_handler)],
            placements: vec![None],
            incoming_edges: vec!(),
            current_mode: Mode::Normal,
            pan: Vec2::ZERO,
            pending_cut: None,
        }
    }

    /// Returns the new node's index.
    fn add_node(&mut self, node: Box<dyn Node>, params: Box<dyn NodeUi>, placement: Option<Placement>) -> usize {
        // Menu-spawned nodes land in the visible area, however far the canvas is panned.
        let placement = placement.or_else(|| {
            let cascade = (self.node_parameters.len() % 10) as f32 * 24.;
            Some(Placement::Initial(pos2(32. + cascade, 96. + cascade) - self.pan))
        });
        self.node_parameters.push(params);
        self.placements.push(placement);
        let _ = self.graph_handler.send(AudioGraphMessage::AddNode(node));
        self.node_parameters.len() - 1
    }

    // TODO make this not 1000 lines long
    /// Render nodes, edges, and ports. Also handles clicks/drags on ports (and background?)
    pub fn render_nodes(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let mut frames: Vec<NodeFrame> = Vec::new();
        let mut duplicate_requests = Vec::new(); // Needed because we can't modify the nodes vec while
                                                 // enumerating it
        let to_global = TSTransform::from_translation(self.pan);

        // Handle clone by dragging
        if let Mode::Cloning(drag) = self.current_mode {
            if ctx.input(|i| i.pointer.primary_down()) {
                if let Some(pointer) = ctx.input(|i| i.pointer.latest_pos()) {
                    let pointer = to_global.inverse() * pointer;
                    self.placements[drag.original] = Some(Placement::Fixed(drag.original_pos));
                    self.placements[drag.clone] = Some(Placement::Fixed(pointer - drag.grab_offset));
                }
            } else {
                self.placements[drag.original] = None;
                self.placements[drag.clone] = None;
                self.current_mode = Mode::Normal;
            }
        }

        for (idx, p) in self.node_parameters.iter_mut().enumerate() {
            // The window's layer is keyed by `Id::new(idx)` (see `node_window`).
            ctx.set_transform_layer(
                egui::LayerId::new(egui::layers::Order::Middle, Id::new(idx)),
                to_global,
            );
            let frame = show_node(ctx, ui, idx, p.as_mut(), self.placements[idx]);

            if frame.response.drag_started() && ctx.input(|i| i.modifiers.alt) {
                duplicate_requests.push((idx, frame.response.rect.min));
            }
            // if matches!(self.current_mode, Mode::Cloning(_)) .is_some_and(|drag| drag.clone == idx) {
            if let Mode::Cloning(dragged) = self.current_mode && dragged.clone == idx { 
                ctx.move_to_top(frame.response.layer_id);
            }

            for (port_idx, port) in frame.inputs.iter().enumerate() {
                if port.clicked || port.dragged {
                    let current_node = (graph::NodeId(idx), graph::PortId(port_idx));
                    match self.current_mode {
                        // Finish creating new edge
                        Mode::ClickingSink(node_id, port) => {
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
                            self.current_mode = Mode::ClickingSource(current_node.0, current_node.1);
                        },
                        _ => {}
                    }
                }
            }

            for (port_idx, port) in frame.outputs.iter().enumerate() {
                if port.clicked {
                    let current_node = (graph::NodeId(idx), graph::PortId(port_idx));
                    match self.current_mode {
                        // Finish creating new edge
                        Mode::ClickingSource(node_id, port) => {
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
                            self.current_mode = Mode::ClickingSink(current_node.0, current_node.1);
                        },
                        _ => {}
                    }
                }
            }
            frames.push(frame);
        }

        for (idx, pos) in duplicate_requests {
            if let Some((node, params)) = self.node_parameters[idx].duplicate() {
                let clone = self.add_node(node, params, Some(Placement::Initial(pos)));
                if let Some(press) = ctx.input(|i| i.pointer.press_origin()) {
                    self.current_mode = Mode::Cloning(CloneDrag {
                        original: idx,
                        clone,
                        original_pos: pos,
                        grab_offset: to_global.inverse() * press - pos,
                    });
                }
                ctx.request_repaint();
            }
        }

        let edge_segment = |edge: &graph::Edge| {
            (
                frames[edge.from.0.0].outputs[edge.from.1.0].pos,
                frames[edge.to.0.0].inputs[edge.to.1.0].pos,
            )
        };

        if let Some((cut_a, cut_b)) = self.pending_cut.take() {
            let mut removed = Vec::new();
            self.incoming_edges.retain(|edge| {
                let (from, to) = edge_segment(edge);
                let hit = segments_intersect(cut_a, cut_b, from, to);
                if hit {
                    removed.push(*edge);
                }
                !hit
            });
            for edge in removed {
                let _ = self.graph_handler.send(AudioGraphMessage::RemoveEdge(edge));
            }
        }

        // Draw edges
        let painter = ctx.layer_painter(egui::LayerId::background());
        for edge in self.incoming_edges.iter() {
            let (from, to) = edge_segment(edge);
            let mut color = Color32::GRAY;
            if let Mode::EdgeKnife(knife_start) = self.current_mode {
                if let Some(knife_end) = ctx.input(|i| i.pointer.latest_pos()) {
                    if segments_intersect(knife_start, knife_end, from, to) {
                        color = Color32::RED;
                    }
                }
            }
            painter.line(vec![from, to], Stroke::new(2.0_f32, color));
        }

        let painter = ctx.layer_painter(egui::LayerId::new(egui::layers::Order::Foreground, Id::new("ephemeral interaction")));
        match self.current_mode {
            Mode::ClickingSource(node_id, port) => {
                let sink_port_pos = frames[node_id.0].inputs[port.0].pos;

                // draw the line
                if let Some(mouse_pos) = ctx.input(|i| i.pointer.latest_pos()) {
                    painter.line(vec![sink_port_pos, mouse_pos], Stroke::new(2.0_f32, Color32::PURPLE));
                }
            }
            Mode::ClickingSink(node_id, port) => {
                let source_port_pos = frames[node_id.0].outputs[port.0].pos;

                // draw the line
                if let Some(mouse_pos) = ctx.input(|i| i.pointer.latest_pos()) {
                    painter.line(vec![source_port_pos, mouse_pos], Stroke::new(2.0_f32, Color32::PURPLE));
                }
            }
            Mode::EdgeKnife(knife_start) => {
                if let Some(knife_end) = ctx.input(|i| i.pointer.latest_pos()) {
                    painter.line_segment([knife_start, knife_end], Stroke::new(2.0_f32, Color32::RED));
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
            let bg_response = ui.interact(ui.max_rect(), bg_id, egui::Sense::click_and_drag());
            if bg_response.clicked() {
                self.current_mode = Mode::Normal;
            }
            // Dragging the background pans, shift-dragging cuts edges
            if bg_response.drag_started() && ctx.input(|i| i.modifiers.shift) {
                if let Some(pos) = ctx.input(|i| i.pointer.press_origin()) {
                    self.current_mode = Mode::EdgeKnife(pos)
                }
            }
            if bg_response.dragged() && matches!(self.current_mode, Mode::Normal) {
                self.pan += bg_response.drag_delta();
            }
            if bg_response.drag_stopped() {
                match self.current_mode {
                    Mode::EdgeKnife(start) => {
                        if let Some(end) = ctx.input(|i| i.pointer.latest_pos()) {
                            self.pending_cut = Some((start, end));
                            self.current_mode = Mode::Normal;
                        }
                    }
                    _ => {}
                }
            }

            ui.heading("infinite recess");

            ui.horizontal(|ui| {
                ui.menu_button("audio", |ui| {
                    ui.set_width(100.0); // To make sure we wrap long text
                    if ui.button("oscillator").clicked() {
                        let (new_osc, new_osc_handler) = OscNode::new();
                        self.add_node(Box::new(new_osc), Box::new(new_osc_handler), None);
                        ui.close();
                    }
                    if ui.button("noise").clicked() {
                        let (new_noise, new_noise_handler) = NoiseNode::new();
                        self.add_node(Box::new(new_noise), Box::new(new_noise_handler), None);
                        ui.close();
                    }
                });

                ui.menu_button("numeric", |ui| {
                    ui.set_width(100.0); // To make sure we wrap long text
                    if ui.button("math").clicked() {
                        let (new_node, node_handler) = MathNode::new();
                        self.add_node(Box::new(new_node), Box::new(node_handler), None);
                        ui.close();
                    }
                });
                ui.menu_button("controller", |ui| {
                    ui.set_width(100.0); // To make sure we wrap long text
                    if ui.button("phasor").clicked() {
                        let (new_osc, new_phasor_handler) = PhaseBender::new();
                        self.add_node(Box::new(new_osc), Box::new(new_phasor_handler), None);
                        ui.close();
                    }
                    if ui.button("metronome").clicked() {
                        let (new_osc, new_metronome_handler) = MetronomeNode::new();
                        self.add_node(Box::new(new_osc), Box::new(new_metronome_handler), None);
                        ui.close();
                    }
                    if ui.button("adsr").clicked() {
                        let (new_osc, new_phasor_handler) = AdsrNode::new();
                        self.add_node(Box::new(new_osc), Box::new(new_phasor_handler), None);
                        ui.close();
                    }
                    if ui.button("sequencer").clicked() {
                        let (new_osc, new_sequencer_handler) = SequencerNode::new();
                        self.add_node(Box::new(new_osc), Box::new(new_sequencer_handler), None);
                        ui.close();
                    }
                });
                ui.menu_button("effects", |ui| {
                    if ui.button("delay").clicked() {
                        let (new_delay, new_delay_handler) = DelayNode::new();
                        self.add_node(Box::new(new_delay), Box::new(new_delay_handler), None);
                        ui.close();
                    }
                    if ui.button("reverb").clicked() {
                        let (new_reverb, new_reverb_handler) = ReverbNode::new();
                        self.add_node(Box::new(new_reverb), Box::new(new_reverb_handler), None);
                        ui.close();
                    }
                });
            });

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                egui::warn_if_debug_build(ui);
            });

            self.render_nodes(ctx, ui);
        });
    }
}
