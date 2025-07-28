use crate::nodes::NodeUi;
use crate::nodes::output::Output;
use std::sync::Arc;

pub struct Canvas {
    output_node: Arc<Output>,
}

impl Canvas {
    /// Called once before the first frame.
    pub fn new(output: Arc<Output>) -> Self {
        Self {
            output_node: output,
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
            for audio_node in self.output_node.get_inputs().iter() {
                // let mut lock = audio_node.try_lock();
                // if let Ok(ref mut node) = lock {
                audio_node.build_controls(&ctx);
                // 
            }
            // ui.separator();


            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                // powered_by_egui_and_eframe(ui);
                egui::warn_if_debug_build(ui);
            });
        });
    }
}
