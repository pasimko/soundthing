use crate::nodes::oscillator::{Oscillator};
use crate::nodes::output::Output;
use crate::nodes::NodeUi;
use std::{sync::{Arc, Mutex}};

pub fn create_gui(output: Arc<Mutex<Output>>) {
    use eframe::wasm_bindgen::JsCast as _;

    // Redirect `log` message to `console.log` and friends:
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();

    let mut nodes : Vec<Arc<Mutex<dyn NodeUi>>> = Vec::new();
    let test_osc = Arc::new(Mutex::new(Oscillator::new()));
    nodes.push(test_osc.clone());
    output.lock().unwrap().attach(test_osc.clone());

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
                Box::new(|_| Ok(Box::new(crate::Canvas::new(nodes)))),
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
