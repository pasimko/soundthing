use crate::nodes::{
    timer::{TimerParameters},
    adsr::{AdsrParameters},
    oscillators::{OscParameters}
};

pub mod oscillators;
pub mod adsr;
pub mod timer;
pub mod output;

// pub const BUFSIZE: usize = 0x1000;
// pub const SAMPLESIZE: usize = 0x100;
pub const SAMPLERATE: usize = 48_000;

pub trait Node {
    fn process(&mut self, output: &mut [f32]);
}

// used by UI to display the parameters of a node
// cloned and decoupled from the actual node's parameters
// but message passing should update them
#[derive(Clone)]
pub enum Parameter {
    Osc(OscParameters),
    Adsr(AdsrParameters),
    Timer(TimerParameters),
}

// I don't love this - added because TimerNode needs to be able
// to send a message to any Node and this seemed simplest
#[derive(Clone)]
pub enum Message {
    Duration(u32),
    Frequency(f32),
    Interval(u32),
    On(bool),
    Volume(u8),
}

pub fn vol_smooth(target: f32, start: f32, n: i32) -> f32 {
    // TODO check if needs to be f64
    let new_vol = target + 0.999_f32.powi(n) * (start - target);
    return new_vol;
}
