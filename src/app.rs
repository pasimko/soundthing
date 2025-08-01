use egui::Id;

use crate::nodes::{oscillators::OscMessage, NodeHandle, Parameter};

// horrible, will fix
pub struct Canvas {
    nodes: Vec<NodeHandle>,
}

impl Canvas {
    /// Called once before the first frame.
    pub fn new(nodes: Vec<NodeHandle>) -> Self {
        let mut node_parameters = Vec::new();
        for _ in 0..nodes.len() {
            node_parameters.push((0, 0));
        }
        Self {
            nodes,
        }
    }
}

// the way to think about this is not as a node, but as a message sender
// all it needs to know is the kind of message it must send
fn render_node(ctx: &egui::Context, handler: &mut NodeHandle, id: usize) {
    let params = &mut handler.params;
    match params {
        Parameter::Osc(p) => {
            egui::Window::new(&p.name).id(Id::new(id)).show(ctx, |ui| {
                let freq_res = ui.add(egui::Slider::new(&mut p.freq, 20.0..=2000.0).text("frequency").logarithmic(true));
                let vol_res = ui.add(egui::Slider::new(&mut p.vol, 0..=101).text("volume"));
                if freq_res.changed() {
                    handler.sender.send(OscMessage::Frequency(p.freq)).unwrap();
                }
                if vol_res.changed() {
                    handler.sender.send(OscMessage::Volume(p.vol)).unwrap();
                }
            });
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

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("music");

            for (i, handler) in (&mut self.nodes).iter_mut().enumerate() {
                render_node(ctx, handler, i);
            }
            // ui.separator();


            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                // powered_by_egui_and_eframe(ui);
                egui::warn_if_debug_build(ui);
            });
        });
    }
}
