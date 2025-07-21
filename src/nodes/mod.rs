pub mod oscillator;
pub mod output;

pub trait Node {
    fn process(&mut self, output: &mut [f32]) -> bool;
}

pub trait NodeUi {
    fn display(&mut self, ctx: &egui::Context);
}
