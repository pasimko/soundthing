mod gui;
mod nodes;
mod wasm_audio;
mod app;
mod dependent_module;
mod graph;

pub use app::Canvas;
use crate::nodes::{Parameter, Node};
use gui::create_gui;
use nodes::oscillators::{SineOsc};
use nodes::output::{Output};
use wasm_audio::wasm_audio;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub async fn web_main() {
    let mut output_node = Output::new();

    let mut msg_handlers: Vec<Parameter> = Vec::new();

    let graph = graph::AudioGraph::new();

    create_gui(msg_handlers);
    let ctx = wasm_audio(output_node).await.unwrap();

    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let body = document.body().unwrap();

    // listen for click event to resume audioContext
    let listener = Closure::<dyn FnMut(_)>::new(move |_: web_sys::Event| {
        drop(ctx.resume().unwrap());
    })
    .into_js_value();
    body.add_event_listener_with_callback("click", listener.as_ref().unchecked_ref())
        .unwrap();
}
