pub mod oscillator;
pub mod output;

pub trait Node {
    fn process(&mut self, output: &mut [f32]) -> bool;
}
