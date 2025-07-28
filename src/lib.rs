mod gui;
mod nodes;
mod wasm_audio;
mod app;
mod dependent_module;

use gui::create_gui;
use nodes::output::{Output};
use wasm_audio::wasm_audio;
use wasm_bindgen::prelude::*;
pub use app::Canvas;
use ringbuf::{traits::*, HeapRb};

#[wasm_bindgen]
pub async fn web_main() {
    // Output is mutable in this thread, maybe?
    // But nothing else can mutate it;
    //
    // The AudioContext can call process, but that won't
    // take a mutable reference: it will get a consumer
    // ring buffer
    //
    // Anything else never gets a reference to our output either
    // they get message passing interfaces, but that's it.
    //
    //
    // yaaay finally getting ring buffer going
    let output_rb = HeapRb::<f32>::new(1024);
    let (mut prod, mut cons) = output_rb.split();

    let mut out = Output::new(prod);
    let ctx = wasm_audio(cons).await.unwrap();
    create_gui(&mut out);
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
    // maybe now a loop that 
}
