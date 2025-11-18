mod app;
mod glue;
mod gui;
mod nodes;

pub use app::Canvas;
use crate::nodes::{Parameter, Node, graph};
use gui::create_gui;
use nodes::oscillators::{SineOsc};
use nodes::output::{Output};
use glue::wasm_audio::wasm_audio;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub async fn web_main() {
    let graph = graph::AudioGraph::new();

    let graph_handler = graph.get_handle();

    create_gui(graph_handler);
    let ctx = wasm_audio(graph).await.unwrap();

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
