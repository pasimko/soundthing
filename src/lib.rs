mod dependent_module;
mod gui;
mod nodes;
mod wasm_audio;
mod app;

use gui::create_gui;
use nodes::output::{Output};
use wasm_audio::wasm_audio;
use wasm_bindgen::prelude::*;
pub use app::Canvas;
use std::{sync::{Arc, Mutex}};

#[wasm_bindgen]
pub async fn web_main() {
    let out = Arc::new(Mutex::new(Output::new()));
    let ctx = wasm_audio(out.clone()).await.unwrap();
    create_gui(out);
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let body = document.body().unwrap();

    let listener = Closure::<dyn FnMut(_)>::new(move |_: web_sys::Event| {
        drop(ctx.resume().unwrap());
    })
    .into_js_value();

    body.add_event_listener_with_callback("click", listener.as_ref().unchecked_ref())
        .unwrap();
}
