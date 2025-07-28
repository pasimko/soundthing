pub mod oscillators;
pub mod output;

pub const BUFSIZE: usize = 0x1000;
pub const SAMPLESIZE: usize = 0x100;

pub trait Node {
    fn process(&mut self);
    fn build_controls(&self, ctx: &egui::Context);
}

pub trait NodeUi {
}
