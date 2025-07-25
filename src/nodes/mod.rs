pub mod oscillators;
pub mod output;

pub trait Node {
    fn process(&mut self, phase: u32, output: &mut [f32]) -> bool;
}

pub trait NodeUi {
    fn build_controls(&mut self, ctx: &egui::Context);
}
