use crate::dependent_module;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::{AudioContext, AudioWorkletNode, AudioWorkletNodeOptions};
use std::sync::{Arc, Mutex};
use crate::nodes::output::Output;

// TODO idk if this "tuple struct" thing is necessary
// or really why it's like this
#[wasm_bindgen]
pub struct WasmAudioProcessor(Arc<Mutex<Output>>);

#[wasm_bindgen]
impl WasmAudioProcessor {
    pub fn process(&mut self, buf: &mut [f32]) -> bool {
        // We can't lock in wasm
        // idk what alternative is...
        // i need to handle the case where it's not ready
        // I guess I should try to lock,
        // the case where 
        // let mut lock = self.0.try_lock();
        // if let Ok(ref mut mutex) = lock {
        //     **mutex = 10;
        // } else {
        //     println!("try_lock failed");
        // }
        match self.0.try_lock() {
            Ok(ref mut node) => node.process(buf),
            Err(_) => false,
        }
    }
    pub fn pack(self) -> usize {
        Box::into_raw(Box::new(self)) as usize
    }
    pub unsafe fn unpack(val: usize) -> Self {
        *Box::from_raw(val as *mut _)
    }
}

// Use wasm_audio if you have a single Wasm audio processor in your application
// whose samples should be played directly. Ideally, call wasm_audio based on
// user interaction. Otherwise, resume the context on user interaction, so
// playback starts reliably on all browsers.
// takes an Output node
// needs to be able to grab a lock when process is called
pub async fn wasm_audio(output: Arc<Mutex<Output>>) -> Result<AudioContext, JsValue> {
    let ctx = AudioContext::new()?;
    prepare_wasm_audio(&ctx).await?;
    let node = wasm_audio_node(&ctx, output)?;
    node.connect_with_audio_node(&ctx.destination())?;
    Ok(ctx)
}

// wasm_audio_node creates an AudioWorkletNode running a Wasm audio processor.
// Remember to call prepare_wasm_audio once on your context before calling
// this function.
pub fn wasm_audio_node(
    ctx: &AudioContext,
    output: Arc<Mutex<Output>>,
) -> Result<AudioWorkletNode, JsValue> {
    let options = AudioWorkletNodeOptions::new();
    options.set_processor_options(Some(&js_sys::Array::of3(
        &wasm_bindgen::module(),
        &wasm_bindgen::memory(),
        &WasmAudioProcessor(output).pack().into(),
    )));
    AudioWorkletNode::new_with_options(ctx, "WasmProcessor", &options)
}

pub async fn prepare_wasm_audio(ctx: &AudioContext) -> Result<(), JsValue> {
    let mod_url = dependent_module!("worklet.js")?;
    JsFuture::from(ctx.audio_worklet()?.add_module(&mod_url)?).await?;
    Ok(())
}
