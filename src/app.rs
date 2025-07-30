use egui::Id;
use std::sync::mpsc::Sender;
use crate::nodes::oscillators::{OscMessage};
use std::iter::zip;

pub struct Canvas {
    node_msg_handlers: Vec<Sender<OscMessage>>,
    node_parameters: Vec<(u32, u8)>,
}

impl Canvas {
    /// Called once before the first frame.
    pub fn new(nodes: Vec<Sender<OscMessage>>) -> Self {
        let mut node_parameters = Vec::new();
        for _ in 0..nodes.len() {
            node_parameters.push((0, 0));
        }
        Self {
            node_msg_handlers: nodes,
            node_parameters,
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

            for (i, (sender, &mut (mut freq, mut vol))) in zip(self.node_msg_handlers.iter(), &mut self.node_parameters.iter_mut()).enumerate() {
                egui::Window::new("test").id(Id::new(i)).show(ctx, |ui| {
                    let freq_res = ui.add(egui::Slider::new(&mut freq, 20..=2000).text("frequency").logarithmic(true));
                    let vol_res = ui.add(egui::Slider::new(&mut vol, 0..=128).text("volume"));
                    if freq_res.changed() {
                        sender.send(OscMessage::Frequency(freq)).unwrap();
                    }
                    if vol_res.changed() {
                        sender.send(OscMessage::Volume(vol)).unwrap();
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
