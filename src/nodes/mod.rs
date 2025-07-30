pub mod oscillators;
pub mod output;

pub const BUFSIZE: usize = 0x1000;
pub const SAMPLESIZE: usize = 0x100;
pub const SAMPLERATE: usize = 48_000;

pub trait Node {
    fn process(&mut self, output: &mut [f32]);
}

pub trait NodeUi {
}
