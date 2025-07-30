use crate::nodes::Node;
use std::sync::Arc;
use std::sync::mpsc::Sender;
use crate::nodes::oscillators::{OscMessage};

pub struct Canvas {
    node_msg_handlers: Vec<Sender<OscMessage>>,
}

impl Canvas {
    /// Called once before the first frame.
    pub fn new(nodes: Vec<Sender<OscMessage>>) -> Self {
        Self {
            node_msg_handlers: nodes,
        }
    }
}

impl eframe::App for Canvas {
    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Put your widgets into a `SidePanel`, `TopBottomPanel`, `CentralPanel`, `Window` or `Area`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:

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
            ui.heading("eframe template");

            // let mut inputs : Vec<u32> = Vec::new();
            // Choose to flash UI when lock fails
            // TODO fix that behavior later
            // TODO move to message passing?
            for audio_node in &self.node_msg_handlers {
                // let mut lock = audio_node.try_lock();
                // if let Ok(ref mut node) = lock {
                egui::Window::new("test").show(ctx, |ui| {
                    // TODO how do I remember these
                    let mut new_freq = 0;
                    let mut new_vol = 0;

                    let freq = ui.add(egui::Slider::new(&mut new_freq, 20..=2000).text("frequency").logarithmic(true));
                    let vol = ui.add(egui::Slider::new(&mut new_vol, 0..=128).text("volume"));
                    if freq.dragged() {
                        audio_node.send(OscMessage::Frequency(new_freq)).unwrap();
                    }
                    if vol.dragged() {
                        audio_node.send(OscMessage::Volume(new_vol)).unwrap();
                    }
                    // self.msg_channel.0.send(new_freq).unwrap();
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
