mod gui;
mod nodes;
mod wasm_audio;
mod app;
mod dependent_module;

pub use app::Canvas;
use crate::nodes::Node;
use crate::nodes::oscillators::OscMessage;
use gui::create_gui;
use nodes::oscillators::SineOsc;
use nodes::output::{Output};
// use ringbuf::{traits::*, HeapRb};
use wasm_audio::wasm_audio;
use wasm_bindgen::prelude::*;
use std::sync::mpsc::Sender;

#[wasm_bindgen]
pub async fn web_main() {
    let mut output_node = Output::new();

    let mut msg_handlers : Vec<Sender<OscMessage>> = Vec::new();

    for _ in 0..3 {
        let (sine_node, sine_node_msg) = SineOsc::new("a");
        output_node.add_input(Box::new(sine_node));
        msg_handlers.push(sine_node_msg);
    }

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
