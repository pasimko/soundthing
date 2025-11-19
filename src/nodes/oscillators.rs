use std::sync::mpsc::Sender;
use crate::nodes::vol_smooth;

use super::{Node, SAMPLERATE, Message};
use std::f32::consts::PI;
use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;

#[derive(Debug, Clone)]
pub struct OscParameters {
    pub freq: f32,
    pub target_vol: u8,
    pub last_vol: f32,
    pub name: String,
    pub sender: Sender<Message>,
}

// TODO why did I choose to make Parameters its own
// thing? OR why did I choose to have phase/vol_tick on their own??
pub struct SineOsc {
    params: OscParameters,
    phase: f32,
    vol_tick: u32,
    msg_receiver: Receiver<Message>,
}

impl SineOsc {
    pub fn new(n: &str) -> (Self, OscParameters) {
        let (msg_sender, msg_receiver) = channel();
        let params = OscParameters {
            freq: 440.,
            target_vol: 100,
            last_vol: 32. / 100.,
            name: n.to_owned(),
            sender: msg_sender,
        };
        let handler = params.clone();
        (Self {
            params,
            phase: 0.,
            vol_tick: 0,
            msg_receiver,
        }, handler)
    }
}

impl Node for SineOsc {
    fn process(&mut self, inputs: &[&[f32]], output: &mut [f32]) {
        // let msg = self.msg_receiver.try_recv();
        // if let Ok(msg) = msg { match msg {
        //     Message::Frequency(val) => self.params.freq = val,
        //     Message::Volume(val) => {
        //         self.vol_tick = 0;
        //         self.params.last_vol = self.params.target_vol as f32 / 100.; // not quite right
        //         self.params.target_vol = val;
        //     },
        //     _ => (),
        // } };
        for sample in output.iter_mut() {
            let frequency = self.params.freq;
            let volume = 100.; //vol_smooth(self.params.target_vol as f32, self.params.last_vol, self.vol_tick as i32);
            *sample = (self.phase).sin() * (volume as f32 / 100.);
            self.phase += 2. * PI / (SAMPLERATE as f32 / frequency);
            self.phase = self.phase.rem_euclid(2. * PI);
            self.vol_tick += 1;
        }
    }
}

pub struct SawtoothOsc {
    params: OscParameters,
    phase: f32,
    msg_receiver: Receiver<Message>,
}

impl SawtoothOsc {
    pub fn new(n: &str) -> (Self, OscParameters) {
        let (msg_sender, msg_receiver) = channel();
        let params = OscParameters {
            freq: 220.,
            target_vol: 32,
            name: n.to_owned(),
            last_vol: 32. / 100.,
            sender: msg_sender,
        };
        let handler = params.clone();
        (Self {
            params,
            phase: 0.,
            msg_receiver,
        }, handler)
    }
}

impl Node for SawtoothOsc {
    fn process(&mut self, inputs: &[&[f32]], output: &mut [f32]) {
        let msg = self.msg_receiver.try_recv();
        if let Ok(msg) = msg { match msg {
            Message::Frequency(val) => self.params.freq = val,
            Message::Volume(val) => self.params.target_vol = val,
            _ => (),
        } };
        for a in output {
            let frequency = self.params.freq;
            let volume = self.params.target_vol;
            self.phase += frequency / 48_000.;
            self.phase = self.phase.rem_euclid(2.);
            *a = (self.phase - 1.) * (volume as f32 / 100.);
        }
    }
}

pub struct SquareOsc {
    params: OscParameters,
    phase: f32,
    msg_receiver: Receiver<Message>,
}

impl SquareOsc {
    pub fn new(n: &str) -> (Self, OscParameters) {
        let (msg_sender, msg_receiver) = channel();
        let params = OscParameters {
            freq: 220.,
            target_vol: 32,
            last_vol: 32. / 100.,
            name: n.to_owned(),
            sender: msg_sender,
        };
        let handler = params.clone();
        (Self {
            params,
            phase: 0.,
            msg_receiver,
        }, handler)
    }
}

impl Node for SquareOsc {
    fn process(&mut self, inputs: &[&[f32]], output: &mut [f32]) {
        let msg = self.msg_receiver.try_recv();
        if let Ok(msg) = msg { match msg {
            Message::Frequency(val) => self.params.freq = val,
            Message::Volume(val) => self.params.target_vol = val,
            _ => (),
        } };
        for a in output {
            let frequency = self.params.freq;
            let volume = self.params.target_vol;
            *a = self.phase.round() * (volume as f32 / 100.);
            self.phase += frequency / 48_000.;
            self.phase = self.phase.rem_euclid(1.);
        }
    }
}


#[cfg(test)]
use wasm_bindgen_test::*;
#[cfg(test)]
mod tests {
    use super::*;
    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_sine() {
        let (mut sine_osc, _) = SineOsc::new("whee");
        let mut output = [0f32; 128];
        sine_osc.process(&[&[]], &mut output);
        assert_eq!(output[0], 0.);
    }
}
//            let frequency = self.params.freq;
//            let volume = vol_smooth(self.params.target_vol as f32, self.params.last_vol, self.vol_tick as i32);
//            self.phase += 2. * PI / (SAMPLERATE as f32 / frequency);
//            self.phase = self.phase.rem_euclid(2. * PI);
//            *sample = (self.phase).sin() * (volume as f32 / 100.);
//            self.vol_tick += 1;
