pub mod oscillators;
pub mod output;

pub trait Node {
    fn process(&mut self, phase: u32, output: &mut [f32]) -> bool;
    fn build_controls(&self, ctx: &egui::Context);
}

pub trait NodeUi {
}
