mod gui;
mod nodes;
mod wasm_audio;
mod app;
mod dependent_module;

pub use app::Canvas;
use crate::nodes::adsr::AdsrNode;
use crate::nodes::timer::TimerNode;
use crate::nodes::{Parameter, Message, Node};
use gui::create_gui;
use nodes::oscillators::{SineOsc, SawtoothOsc, SquareOsc};
use nodes::output::{Output};
use wasm_audio::wasm_audio;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub async fn web_main() {
    let mut output_node = Output::new();

    let mut msg_handlers : Vec<Parameter> = Vec::new();

    let (node, node_handler) = SineOsc::new("Sine Osc");
    output_node.add_input(Box::new(node));
    msg_handlers.push(Parameter::Osc(node_handler));

    let (mut adsr_node, node_handler) = AdsrNode::new("ADSR");
    msg_handlers.push(Parameter::Adsr(node_handler));

    let (mut timer_node, node_handler) = TimerNode::new("timer", Message::Frequency(220.), Message::Frequency(220.*1.5));
    msg_handlers.push(Parameter::Timer(node_handler));

    let (node, node_handler) = SineOsc::new("Sine Osc (timer)");
    timer_node.add_sink(node_handler.sender.clone());
    output_node.add_input(Box::new(node));
    msg_handlers.push(Parameter::Osc(node_handler));

    let (node, node_handler) = SineOsc::new("Sine Osc (adsr)");
    adsr_node.add_input(Box::new(node));
    msg_handlers.push(Parameter::Osc(node_handler));

    // let (node, node_handler) = SquareOsc::new("Square Osc");
    // msg_handlers.push(node_handler);

    output_node.add_input(Box::new(adsr_node));
    output_node.add_input(Box::new(timer_node));

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
