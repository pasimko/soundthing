use std::sync::mpsc::Receiver;

use crate::nodes::{
    adsr::{AdsrParameters},
    delay::*,
    oscillators::*,
    // harmonics::*,
    output::*,
    phasor::*,
    graph::{PortId},
    math::*,
    metronome::*,
    noise::*,
    reverb::*,
    sequencer::*,
};

// pub mod timer;
pub mod adsr;
pub mod delay;
pub mod math;
pub mod metronome;
pub mod noise;
pub mod oscillators;
pub mod output;
pub mod phasor;
pub mod reverb;
pub mod sequencer;
// pub mod harmonics;

pub mod graph;

// pub const BUFSIZE: usize = 0x1000;
// pub const SAMPLESIZE: usize = 0x100;
pub const SAMPLERATE: usize = 48_000;

pub trait Node {
    /// How many output ports the node has; `process` is given one buffer for each.
    fn output_count(&self) -> usize {
        1
    }

    fn process(&mut self, inputs: &[(PortId, &[f32])], outputs: &mut [Vec<f32>]);
}

/// Used by UI to display the parameters of a node
// cloned and decoupled from the actual node's parameters
// but message passing should keep them synced.
#[derive(Clone)]
pub enum NodeParameter {
    Adsr(AdsrParameters),
    Delay(DelayParameters),
    Math(MathParameters),
    Metronome(MetronomeParameters),
    Osc(OscParameters),
    Output(OutputParameters),
    Phasor(PhasorParameters),
    Reverb(ReverbParameters),
    Sequencer(SequencerParameters),
    Noise(NoiseParameters),
}

/// Applies every pending message
pub fn drain_messages<M>(rx: &Receiver<M>, mut handle: impl FnMut(M)) {
    while let Ok(msg) = rx.try_recv() {
        handle(msg);
    }
}

/// Where a node window goes, in the window layer's own (pre-pan) coordinates.
#[derive(Clone, Copy)]
pub enum Placement {
    /// Where the window first appears; ignored once it exists.
    Initial(egui::Pos2),
    /// Pinned here, and not draggable, for as long as it's passed.
    Fixed(egui::Pos2),
}

pub trait NodeUi {
    fn title(&self) -> String;

    /// The ports drawn beside the window. Port `i` here is `PortId(i)` in the graph, so the
    /// order must match how `process` reads its inputs.
    fn ports(&self) -> PortDescriptions;

    /// The window's contents.
    fn draw(&mut self, ui: &mut egui::Ui);

    /// An independent, unwired copy of this node, or `None` if the node can't be copied.
    fn duplicate(&self) -> Option<(Box<dyn Node>, Box<dyn NodeUi>)> {
        None
    }
}

/// A connection point drawn beside a node's window.
pub struct Port {
    /// Screen space, regardless of how far the window's layer is panned.
    pub pos: egui::Pos2,
    pub clicked: bool,
    pub drag_started: bool,
    pub hovered: bool,
}

pub struct NodeFrame {
    pub response: egui::Response,
    pub inputs: Vec<Port>,
    pub outputs: Vec<Port>,
}

const PORT_RADIUS: f32 = 5.5;
pub const PORT_HOVER_RADIUS: f32 = 7.0;
const PORT_SPACING: f32 = 18.0;
// how far outside the window's edge a port sits
const PORT_OFFSET: f32 = 10.0;
pub const PORT_HIT_SLOP: f32 = 8.0;

// egui clips to the screen rect in a layer's own coordinates, which hides anything that
// has been panned past the original screen edge, so layers draw with this clip instead.
fn unbounded_rect() -> egui::Rect {
    egui::Rect::from_center_size(egui::Pos2::ZERO, egui::Vec2::splat(1.0e6))
}

fn node_window<'a>(title: String, idx: usize, placement: Option<Placement>) -> egui::Window<'a> {
    // Windows live in a panned layer. egui clips a window to its constrain rect even when
    // `constrain` is off, so the default (the screen rect) would hide it once panned
    // off-screen.
    let window = egui::Window::new(title)
        .id(egui::Id::new(idx))
        .constrain_to(unbounded_rect())
        .constrain(false);
    match placement {
        Some(Placement::Initial(pos)) => window.default_pos(pos),
        Some(Placement::Fixed(pos)) => window.fixed_pos(pos),
        None => window,
    }
}

/// The contents of a port's tooltip.
pub fn port_tooltip(ui: &mut egui::Ui, info: &PortInfo, is_input: bool) {
    ui.strong(info.name);
    ui.label(info.description);
    ui.weak(info.connections_text(is_input));
}

/// Shows a node's window and its ports. `ui` is the canvas the ports' clicks are sensed in.
pub fn show_node(
    ctx: &egui::Context,
    ui: &egui::Ui,
    idx: usize,
    node: &mut dyn NodeUi,
    placement: Option<Placement>,
) -> NodeFrame {
    let response = node_window(node.title(), idx, placement)
        .show(ctx, |ui| node.draw(ui))
        .unwrap()
        .response;
    let ports = node.ports();

    // The window may sit in a panned layer: dots are painted in the layer's own space, but
    // clicks and the positions handed back are in screen space.
    let to_global = ctx
        .layer_transform_to_global(response.layer_id)
        .unwrap_or(egui::emath::TSTransform::IDENTITY);
    let mut painter = ctx.layer_painter(response.layer_id);
    painter.set_clip_rect(unbounded_rect());

    let port = |is_input: bool, port_idx: usize, info: &PortInfo, local_pos: egui::Pos2| {
        let pos = to_global * local_pos;
        let hit = ui.interact(
            egui::Rect::from_pos(pos).expand(PORT_HIT_SLOP),
            response.id.with((is_input, port_idx)),
            egui::Sense::click_and_drag(),
        ).on_hover_ui(|ui| port_tooltip(ui, info, is_input));
        let radius = if hit.hovered() { PORT_HOVER_RADIUS } else { PORT_RADIUS };
        painter.circle_filled(local_pos, radius, egui::Color32::BLACK);
        Port { pos, clicked: hit.clicked(), drag_started: hit.drag_started(), hovered: hit.contains_pointer() }
    };
    let row = |port_idx: usize| PORT_RADIUS * 2. + PORT_SPACING * port_idx as f32;

    let left = response.rect.left_top();
    let right = response.rect.right_top();
    let inputs = ports.inputs.iter().enumerate()
        .map(|(i, info)| port(true, i, info, egui::pos2(left.x - PORT_OFFSET, left.y + row(i))))
        .collect();
    let outputs = ports.outputs.iter().enumerate()
        .map(|(i, info)| port(false, i, info, egui::pos2(right.x + PORT_OFFSET, right.y + row(i))))
        .collect();

    NodeFrame { response, inputs, outputs }
}

/// How many edges a port is meant to have.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Connections {
    One,
    Many,
}

pub struct PortInfo {
    pub name: &'static str,
    pub description: &'static str,
    pub connections: Connections,
}

impl PortInfo {
    /// An input that takes a single connection.
    pub fn input(name: &'static str, description: &'static str) -> Self {
        Self { name, description, connections: Connections::One }
    }

    /// An input that takes any number of connections.
    pub fn input_many(name: &'static str, description: &'static str) -> Self {
        Self { name, description, connections: Connections::Many }
    }

    /// An output, which can feed any number of inputs.
    pub fn output(name: &'static str, description: &'static str) -> Self {
        Self { name, description, connections: Connections::Many }
    }

    fn connections_text(&self, is_input: bool) -> &'static str {
        match (is_input, self.connections) {
            (true, Connections::One) => "Takes one connection",
            (true, Connections::Many) => "Takes any number of connections",
            (false, _) => "Can feed any number of inputs",
        }
    }
}

pub struct PortDescriptions {
    pub inputs: Vec<PortInfo>,
    pub outputs: Vec<PortInfo>,
}

impl PortDescriptions {
    pub fn new() -> Self {
        Self {
            inputs: Vec::new(),
            outputs: Vec::new(),
        }
    }
    pub fn with_ports(inputs: Vec<PortInfo>, outputs: Vec<PortInfo>) -> Self {
        Self {
            inputs,
            outputs,
        }
    }
}

pub fn vol_smooth(target: f32, start: f32, n: i32) -> f32 {
// TODO check if needs to be f64
    
    target + 0.999_f32.powi(n) * (start - target)
}

