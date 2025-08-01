mod gui;
mod nodes;
mod wasm_audio;
mod app;
mod dependent_module;

pub use app::Canvas;
use crate::nodes::adsr::AdsrNode;
use crate::nodes::Node;
use gui::create_gui;
use nodes::oscillators::{SineOsc, SawtoothOsc, SquareOsc};
// use nodes::adsr::{AdsrNode};
use nodes::output::{Output};
use wasm_audio::wasm_audio;
use wasm_bindgen::prelude::*;
use crate::nodes::{NodeHandle};

#[wasm_bindgen]
pub async fn web_main() {
    let mut output_node = Output::new();

    let mut msg_handlers : Vec<NodeHandle> = Vec::new();

    let (node, node_handler) = SineOsc::new("Sine Osc");
    output_node.add_input(Box::new(node));
    msg_handlers.push(node_handler);

    let (mut adsr_node, node_handler) = AdsrNode::new("ADSR");
    msg_handlers.push(node_handler);

    let (node, node_handler) = SawtoothOsc::new("Sawtooth Osc");
    output_node.add_input(Box::new(node));
    msg_handlers.push(node_handler);

    let (node, node_handler) = SquareOsc::new("Square Osc");
    adsr_node.add_input(Box::new(node));
    msg_handlers.push(node_handler);

    output_node.add_input(Box::new(adsr_node));

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
