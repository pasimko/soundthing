use std::sync::mpsc::Sender;

use egui::Id;

use crate::nodes::{Message, Parameter, graph::AudioGraphMessage};

pub struct Canvas {
    graph_handler: Sender<AudioGraphMessage>,
}

impl Canvas {
    /// Called once before the first frame.
    pub fn new(graph_handler: Sender<AudioGraphMessage>) -> Self {
        Self {
            graph_handler,
        }
    }
}

// the way to think about this is not as a node, but as a message sender
// all it needs to know is the kind of message it must send
// TODO refactor. This could just take an array of tuples or something: key, type, range?
fn render_node(ctx: &egui::Context, handler: &mut Parameter, id: usize) {
    let params = handler;
    match params {
        Parameter::Osc(p) => {
            let result = egui::Window::new(&p.name).id(Id::new(id)).show(ctx, |ui| {
                let freq_res = ui.add(egui::Slider::new(&mut p.freq, 20.0..=2000.0).text("frequency").logarithmic(true));
                let vol_res = ui.add(egui::Slider::new(&mut p.target_vol, 0..=101).text("volume"));
                if freq_res.changed() {
                    p.sender.send(Message::Frequency(p.freq)).unwrap();
                }
                if vol_res.changed() {
                    p.sender.send(Message::Volume(p.target_vol)).unwrap();
                }
            });
            if let Some(r) = result {
                if r.response.contains_pointer() {
                    ctx.input(|i| {
                        for &key in i.keys_down.iter() {
                            if i.key_pressed(key) {
                                p.freq = match key {
                                    egui::Key::A => 220.,
                                    egui::Key::S => 220.*1.25,
                                    egui::Key::D => 220.*1.5,
                                    egui::Key::F => 220.*1.75,
                                    egui::Key::G => 220.*2.,
                                    _ => 110.,
                                };
                                p.sender.send(Message::Frequency(p.freq)).unwrap();
                            }
                        }
                    });
                }
            }
        }
        // Parameter::Adsr(p) => {
        //     egui::Window::new(&p.name).id(Id::new(id)).show(ctx, |ui| {
        //         let on = ui.add(egui::Button::new("on"));
        //         let on = on.hovered();
        //         p.on = on;
        //         p.sender.send(Message::On(on)).unwrap();
        //     });
        // }
        // Parameter::Timer(p) => {
        //     egui::Window::new(&p.name).id(Id::new(id)).show(ctx, |ui| {
        //         // let on = ui.add(egui::Button::new("on"));
        //         // let on = on.hovered();
        //         // p.on = on;
        //         // p.sender.send(AdsrMessage::On(on)).unwrap();
        //     });
        // }
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
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("music");

            // ui.separator();


            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                // powered_by_egui_and_eframe(ui);
                egui::warn_if_debug_build(ui);
            });
        });
    }
}
