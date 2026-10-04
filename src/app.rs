use std::sync::mpsc::Sender;
use super::graph;
use egui::{Id, Shape};
use egui::{
    Color32, Pos2, Stroke, Vec2, pos2,
    emath::TSTransform,
};


use crate::nodes::{*, oscillators::*, adsr::AdsrNode, graph::AudioGraphMessage, output::OutputNode,
math::MathNode, metronome::MetronomeNode, sequencer::SequencerNode, noise::NoiseNode,
phasor::PhaseBender, delay::DelayNode, reverb::ReverbNode, rhythm::RhythmNode, pulsewidth::PulsewidthNode, interpolator::InterpolatorNode};

enum Mode {
    Normal,
    Connecting { anchor: PortRef, dragging: bool },
    EdgeKnife(Pos2),
    Cloning(CloneDrag)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    Input,
    Output,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct PortRef {
    node: graph::NodeId,
    port: graph::PortId,
    side: Side,
}

// How close a dragged edge has to get to a port to snap to it
const SNAP_RADIUS: f32 = 14.0;

/// The edge joining an output and an input, whichever order they're given in. `None` if they're
/// on the same side or on the same node.
impl PortRef {
    fn address(self) -> graph::PortAddress {
        graph::PortAddress { node: self.node, port: self.port }
    }
}

/// Returns a valid edge construction between two ports, if possible
fn edge_between(a: PortRef, b: PortRef) -> Option<graph::Edge> {
    if a.node == b.node {
        return None;
    }
    match (a.side, b.side) {
        (Side::Output, Side::Input) => Some(graph::Edge { from: a.address(), to: b.address() }),
        (Side::Input, Side::Output) => Some(graph::Edge { from: b.address(), to: a.address() }),
        _ => None,
    }
}

fn port_pos(frames: &[NodeFrame], port: PortRef) -> Pos2 {
    let frame = &frames[port.node.0];
    match port.side {
        Side::Input => frame.inputs[port.port.0].pos,
        Side::Output => frame.outputs[port.port.0].pos,
    }
}

/// The closest port to `pointer`, within snapping range, that `anchor` could connect to.
fn port_near(frames: &[NodeFrame], anchor: PortRef, pointer: Pos2) -> Option<PortRef> {
    let mut best: Option<(f32, PortRef)> = None;
    for (node, frame) in frames.iter().enumerate() {
        for (side, ports) in [(Side::Input, &frame.inputs), (Side::Output, &frame.outputs)] {
            for (port, p) in ports.iter().enumerate() {
                let candidate = PortRef { node: graph::NodeId(node), port: graph::PortId(port), side };
                let dist = p.pos.distance(pointer);
                if dist <= SNAP_RADIUS
                    && edge_between(anchor, candidate).is_some()
                    && best.is_none_or(|(d, _)| dist < d)
                {
                    best = Some((dist, candidate));
                }
            }
        }
    }
    best.map(|(_, port)| port)
}

pub struct Canvas {
    graph_handler: Sender<AudioGraphMessage>,
    node_parameters: Vec<Box<dyn NodeUi>>,
    // where each node's window goes; parallel to `node_parameters`
    placements: Vec<Option<Placement>>,
    edges: Vec<graph::Edge>,
    current_mode: Mode,
    // offset applied to every node window, in screen points
    pan: Vec2,
    // since knife drags are finished outside of the edge drawing loop, this saves the cut for
    // when edge positions are known
    pending_cut: Option<(Pos2, Pos2)>,
    // the port an edge drag is snapped to, and when it got there. egui only reports hovers for
    // the dragged port, so the usual port tooltips can't appear for the drop target.
    drag_hover: Option<(PortRef, f64)>,
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
            edges: vec!(),
            current_mode: Mode::Normal,
            pan: Vec2::ZERO,
            pending_cut: None,
            drag_hover: None,
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

    /// Select edges going to a node that should take just one incoming edge
    fn edges_displaced_by(&self, edge: graph::Edge) -> Vec<graph::Edge> {
        let takes_one = self.node_parameters[edge.to.node.0].ports().inputs[edge.to.port.0].connections == Connections::One;
        if takes_one {
            self.edges.iter().copied().filter(|e| e.to == edge.to && *e != edge).collect()
        }
        else {
            vec!()
        }
    }

    /// Add or replace an edge
    fn connect(&mut self, edge: graph::Edge) {
        if self.edges.contains(&edge) {
            return;
        }
        // pick the node parameter that our edge is going to
        // get its port descriptions
        // get the input port_info that we're going to
        // get how many connections it takes
        let replaced = self.edges_displaced_by(edge);
        self.edges.retain(|e| !replaced.contains(e));
        for old in replaced {
            let _ = self.graph_handler.send(AudioGraphMessage::RemoveEdge(old));
        }
        
        self.edges.push(edge);
        let _ = self.graph_handler.send(AudioGraphMessage::AddEdge(edge));
    }

    /// Render nodes, edges, and ports. Also handles clicks/drags on ports
    // TODO make not one million lines long holy moly this just keeps getting worse
    pub fn render_nodes(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let mut frames: Vec<NodeFrame> = Vec::new();
        let to_global = TSTransform::from_translation(self.pan);

        // Needed because we can't modify the nodes/edge vecs while enumerating them
        let mut new_node_requests = Vec::new();
        let mut new_edge_requests = Vec::new();

        // the compatible port being hovered while connecting by clicking
        let mut click_hover = None;

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

        // Render each node and its ports
        for (idx, p) in self.node_parameters.iter_mut().enumerate() {
            // The window's layer is keyed by `Id::new(idx)` (see `node_window`).
            ctx.set_transform_layer(
                egui::LayerId::new(egui::layers::Order::Middle, Id::new(idx)),
                to_global,
            );
            let frame = show_node(ctx, ui, idx, p.as_mut(), self.placements[idx]);

            if frame.response.drag_started() && ctx.input(|i| i.modifiers.alt) {
                new_node_requests.push((idx, frame.response.rect.min));
            }
            if let Mode::Cloning(dragged) = self.current_mode && dragged.clone == idx { 
                ctx.move_to_top(frame.response.layer_id);
            }

            // Handle port interactions
            for (side, ports) in [(Side::Input, &frame.inputs), (Side::Output, &frame.outputs)] {
                for (port_idx, port) in ports.iter().enumerate() {
                    let this = PortRef { node: graph::NodeId(idx), port: graph::PortId(port_idx), side };
                    if port.drag_started {
                        if matches!(self.current_mode, Mode::Normal | Mode::Connecting { .. }) {
                            self.current_mode = Mode::Connecting { anchor: this, dragging: true };
                        }
                    }
                    else if port.clicked {
                        match self.current_mode {
                            // Start creating new edge
                            Mode::Normal => {
                                self.current_mode = Mode::Connecting { anchor: this, dragging: false };
                            },
                            // Clicking the same port again cancels
                            Mode::Connecting { anchor, .. } if anchor == this => {
                                self.current_mode = Mode::Normal;
                            },
                            // Finish creating new edge
                            Mode::Connecting { anchor, .. } => {
                                if let Some(edge) = edge_between(anchor, this) {
                                    new_edge_requests.push(edge);
                                    self.current_mode = Mode::Normal;
                                }
                            },
                            _ => {}
                        }
                    }
                    else if port.hovered {
                        if let Mode::Connecting { anchor, dragging: false } = self.current_mode {
                            if edge_between(anchor, this).is_some() {
                                click_hover = Some(this);
                            }
                        }
                    }
                }
            }
            frames.push(frame);
        }

        for (idx, pos) in new_node_requests {
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

        // Finish or cancel an edge drag now that every port's position is known
        if let Mode::Connecting { anchor, dragging: true, .. } = self.current_mode {
            if !ctx.input(|i| i.pointer.primary_down()) {
                let target = ctx.input(|i| i.pointer.latest_pos())
                    .and_then(|pointer| port_near(&frames, anchor, pointer));
                if let Some(edge) = target.and_then(|target| edge_between(anchor, target)) {
                    new_edge_requests.push(edge);
                }
                self.current_mode = Mode::Normal;
            }
        }
        if matches!(self.current_mode, Mode::Connecting { .. }) && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.current_mode = Mode::Normal;
        }
        for edge in new_edge_requests {
            self.connect(edge);
        }

        let edge_segment = |edge: &graph::Edge| {
            (
                frames[edge.from.node.0].outputs[edge.from.port.0].pos,
                frames[edge.to.node.0].inputs[edge.to.port.0].pos,
            )
        };

        if let Some((cut_a, cut_b)) = self.pending_cut.take() {
            let mut removed = Vec::new();
            self.edges.retain(|edge| {
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

        let painter = ctx.layer_painter(egui::LayerId::background());
        let ephemeral_layer = egui::LayerId::new(egui::layers::Order::Foreground, Id::new("ephemeral interaction"));
        let ephemeral_painter = ctx.layer_painter(ephemeral_layer);
        // the edge that connecting right now would make; its displaced edges are drawn red below
        let mut potential_edge = None;
        match self.current_mode {
            Mode::Connecting { anchor, dragging } => {
                if let Some(pointer) = ctx.input(|i| i.pointer.latest_pos()) {
                    // The line snaps onto a port it could connect to: the nearest one while
                    // dragging, the hovered one while clicking
                    let target = if dragging { port_near(&frames, anchor, pointer) } else { click_hover };
                    let end = target.map_or(pointer, |target| port_pos(&frames, target));
                    ephemeral_painter.line(vec![port_pos(&frames, anchor), end], Stroke::new(2.0_f32, Color32::PURPLE));
                    if let Some(target) = target {
                        ephemeral_painter.circle_filled(end, PORT_HOVER_RADIUS, Color32::BLACK);
                        potential_edge = edge_between(anchor, target);
                    }
                    // Manually show tooltips since there's no hover detection in drag mode
                    let now = ctx.input(|i| i.time);
                    let target = target.filter(|_| dragging);
                    match (target, self.drag_hover) {
                        (Some(target), Some((hovered, _))) if target == hovered => {}
                        (Some(target), _) => self.drag_hover = Some((target, now)),
                        (None, _) => self.drag_hover = None,
                    }
                    if let Some((hovered, since)) = self.drag_hover {
                        let delay = ctx.style().interaction.tooltip_delay as f64;
                        if now - since < delay {
                            ctx.request_repaint_after_secs((since + delay - now) as f32);
                        } else {
                            let is_input = hovered.side == Side::Input;
                            let ports = self.node_parameters[hovered.node.0].ports();
                            let info = if is_input {
                                &ports.inputs[hovered.port.0]
                            } else {
                                &ports.outputs[hovered.port.0]
                            };
                            let port_rect = egui::Rect::from_center_size(
                                port_pos(&frames, hovered),
                                Vec2::splat(PORT_HOVER_RADIUS * 2.),
                            );
                            egui::Tooltip::always_open(ctx.clone(), ephemeral_layer, Id::new("edge drag tooltip"), port_rect)
                                .show(|ui| port_tooltip(ui, info, is_input));
                        }
                    }
                }
            }
            Mode::EdgeKnife(knife_start) => {
                if let Some(knife_end) = ctx.input(|i| i.pointer.latest_pos()) {
                    ephemeral_painter.add(Shape::dashed_line(&[knife_start, knife_end], Stroke::new(2.0_f32, Color32::RED), 6.0, 2.5));
                }
            }
            _ => {}
        }

        // Draw edges
        for edge in self.edges.iter() {
            let (from, to) = edge_segment(edge);
            let mut color = Color32::GRAY;
            match self.current_mode {
                Mode::EdgeKnife(knife_start) => {
                    if let Some(knife_end) = ctx.input(|i| i.pointer.latest_pos()) {
                        if segments_intersect(knife_start, knife_end, from, to) {
                            color = Color32::RED;
                        }
                    }
                }
                Mode::Connecting { .. } => {
                    if let Some(potential_edge) = potential_edge {
                        if self.edges_displaced_by(potential_edge).contains(edge) {
                            color = Color32::RED;
                        }
                    }
                }
                _ => {}

            }
            painter.line(vec![from, to], Stroke::new(2.0_f32, color));
        }

        // This feels like an out-of-place check
        if !matches!(self.current_mode, Mode::Connecting { dragging: true, .. }) {
            self.drag_hover = None;
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

        let _panel_response = egui::CentralPanel::default().show(ctx, |ui| {
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
                    if ui.button("interpolator").clicked() {
                        let (new_node, node_handler) = InterpolatorNode::new();
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
                    if ui.button("rhythm").clicked() {
                        let (new_rhythm, new_rhythm_handler) = RhythmNode::new();
                        self.add_node(Box::new(new_rhythm), Box::new(new_rhythm_handler), None);
                        ui.close();
                    }
                    if ui.button("pulsewidth").clicked() {
                        let (new_pulsewidth, new_pulsewidth_handler) = PulsewidthNode::new();
                        self.add_node(Box::new(new_pulsewidth), Box::new(new_pulsewidth_handler), None);
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
