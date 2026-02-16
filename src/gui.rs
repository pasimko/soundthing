use std::sync::mpsc::Sender;

use crate::{nodes::graph::AudioGraphMessage, Canvas};

// Helper functions to create base visuals
fn base_visuals(theme_visuals: egui::Visuals, widget: egui::style::WidgetVisuals) -> egui::Visuals {
    egui::Visuals {
        window_shadow: egui::Shadow::NONE,
        window_corner_radius: egui::CornerRadius::ZERO,
        menu_corner_radius: egui::CornerRadius::ZERO,
        popup_shadow: egui::Shadow::NONE,
        widgets: base_widget_visuals(widget, &theme_visuals),
        ..theme_visuals
    }
}

fn base_widget_visuals(widget: egui::style::WidgetVisuals, theme_visuals: &egui::Visuals) -> egui::style::Widgets {
    let widgets = &theme_visuals.widgets;
    egui::style::Widgets {
        noninteractive: egui::style::WidgetVisuals {
            weak_bg_fill: egui::Color32::RED,
            corner_radius: egui::CornerRadius::ZERO,
            ..widgets.noninteractive
        },
        inactive: egui::style::WidgetVisuals {
            weak_bg_fill: theme_visuals.panel_fill,
            bg_stroke: theme_visuals.window_stroke,
            corner_radius: egui::CornerRadius::ZERO,
            ..widgets.inactive
        },
        hovered: egui::style::WidgetVisuals {
            corner_radius: egui::CornerRadius::ZERO,
            ..widgets.hovered
        },
        active: egui::style::WidgetVisuals {
            corner_radius: egui::CornerRadius::ZERO,
            ..widgets.active
        },
        open: egui::style::WidgetVisuals {
            corner_radius: egui::CornerRadius::ZERO,
            weak_bg_fill: widget.weak_bg_fill,
            ..widgets.open
        },
    }
}

pub fn create_gui(graph_handler: Sender<AudioGraphMessage>) {
    use eframe::wasm_bindgen::JsCast as _;

    // Redirect `log` message to `console.log` and friends:
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();

    wasm_bindgen_futures::spawn_local(async move {
        let canvas = document
            .get_element_by_id("the_canvas_id")
            .expect("Failed to find the_canvas_id")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("the_canvas_id was not a HtmlCanvasElement");

        let start_result = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(move |cc| {
                    // Set fonts
                    let mut fonts = egui::FontDefinitions::default();
                    fonts.font_data.insert("ArvoRegular".to_owned(),
                        std::sync::Arc::new(
                            egui::FontData::from_static(include_bytes!("../assets/fonts/Arvo-Regular.ttf"))
                        )
                    );

                    fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap()
                        .insert(0, "ArvoRegular".to_owned());

                    cc.egui_ctx.set_fonts(fonts);

                    // Widget theming
                    let dark_widget = egui::style::WidgetVisuals {
                        weak_bg_fill: egui::Color32::LIGHT_YELLOW,
                        ..egui::Visuals::dark().widgets.inactive
                    };
                    let light_widget = egui::style::WidgetVisuals {
                        weak_bg_fill: egui::Color32::LIGHT_YELLOW,
                        ..egui::Visuals::light().widgets.inactive
                    };
                    cc.egui_ctx.set_visuals_of(
                        egui::Theme::Dark,
                        egui::Visuals {
                            faint_bg_color: egui::Color32::DARK_BLUE,
                            ..base_visuals(egui::Visuals::dark(), dark_widget)
                        }
                    );
                    cc.egui_ctx.set_visuals_of(
                        egui::Theme::Light,
                        egui::Visuals {
                            faint_bg_color: egui::Color32::LIGHT_BLUE,
                            ..base_visuals(egui::Visuals::light(), light_widget)
                        }
                    );

                    cc.egui_ctx.all_styles_mut(|style| {
                        style.animation_time = 0.05;
                    });

                    Ok(Box::new(Canvas::new(graph_handler)))
                }),
                )
                    .await;

        // Remove the loading text and spinner:
        if let Some(loading_text) = document.get_element_by_id("loading_text") {
            match start_result {
                Ok(_) => {
                    loading_text.remove();
                }
                Err(e) => {
                    loading_text.set_inner_html(
                        "<p> The app has crashed. See the developer console for details. </p>",
                    );
                    panic!("Failed to start eframe: {e:?}");
                }
            }
        }
    });
}
