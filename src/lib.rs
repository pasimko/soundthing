mod dependent_module;
mod gui;
mod nodes;
mod wasm_audio;
mod app;

use gui::create_gui;
use nodes::oscillator::{Oscillator, OscParams};
use crate::nodes::Node;
use nodes::output::{Output};
use wasm_audio::wasm_audio;
use wasm_bindgen::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
pub use app::Canvas;

#[wasm_bindgen]
pub async fn web_main() {
    // On the application level, audio worklet internals are abstracted by wasm_audio:
    // let out_params: &'static OutputParams = Box::leak(Box::default());
    // idk if rc is gonna work
    let osc = Rc::new(RefCell::new(Oscillator::new()));
    let mut out = Output::new();
    out.attach(&mut *osc.borrow_mut());
    let ctx = wasm_audio(Box::new(move |buf| {
        out.process(buf)
    }))
    .await
    .unwrap();
    // create_gui(params, ctx);
}
