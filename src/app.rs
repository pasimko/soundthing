use egui::Id;
use std::sync::mpsc::Sender;
use crate::nodes::oscillators::{OscMessage, OscNodeHandle};
use std::iter::zip;

// horrible, will fix
pub struct Canvas {
    nodes: Vec<OscNodeHandle>,
}

impl Canvas {
    /// Called once before the first frame.
    pub fn new(nodes: Vec<OscNodeHandle>) -> Self {
        let mut node_parameters = Vec::new();
        for _ in 0..nodes.len() {
            node_parameters.push((0, 0));
        }
        Self {
            nodes,
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

            // horrible
            for (i, handler) in (&mut self.nodes).iter_mut().enumerate() {
                let params = &mut handler.params;
                egui::Window::new(&params.name).id(Id::new(i)).show(ctx, |ui| {
                    let freq_res = ui.add(egui::Slider::new(&mut params.freq, 20.0..=2000.0).text("frequency").logarithmic(true));
                    let vol_res = ui.add(egui::Slider::new(&mut params.vol, 0..=128).text("volume"));
                    if freq_res.changed() {
                        handler.sender.send(OscMessage::Frequency(params.freq)).unwrap();
                    }
                    if vol_res.changed() {
                        handler.sender.send(OscMessage::Volume(params.vol)).unwrap();
                    }
                });
            }
            // ui.separator();


            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                // powered_by_egui_and_eframe(ui);
                egui::warn_if_debug_build(ui);
            });
        });
    }
}
