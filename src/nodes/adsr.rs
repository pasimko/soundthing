use super::{Node, Parameter, Message};
use std::sync::mpsc::channel;
use std::sync::mpsc::Sender;
use std::sync::mpsc::Receiver;
use std::iter::zip;

#[derive(Debug, Clone)]
pub struct AdsrParameters {
    pub on: bool,
    pub name: String,
    pub sender: Sender<Message>,
}

pub struct AdsrNode {
    params: AdsrParameters,
    msg_receiver: Receiver<Message>,
    inputs: Vec<Box<dyn Node>>,
    on_count: u8,
}

impl AdsrNode {
    pub fn new(n: &str) -> (Self, AdsrParameters) {
        let (msg_sender, msg_receiver) = channel();
        let params = AdsrParameters {
            on: false,
            name: n.to_owned(),
            sender: msg_sender,
        };
        let handler = params.clone();
        (Self {
            params,
            msg_receiver,
            inputs: Vec::new(),
            on_count: 0,
        }, handler)
    }
    pub fn add_input(&mut self, node: Box<dyn Node>) {
        self.inputs.push(node);
    }
}

// impl Node for AdsrNode {
//     fn process(&mut self, output: &mut [f32]) {
//         let msg = self.msg_receiver.try_recv();
//         if let Ok(msg) = msg { match msg {
//             Message::On(val) => self.params.on = val,
//             _ => (),
//         } };
//         if self.params.on {
//             if self.on_count < 100 {
//                 self.on_count += 1;
//             }
//             output.iter_mut().for_each( |x| *x = 0.);
//             for node in &mut self.inputs {
//                 let mut cur_buf = output.to_vec(); // pointless but
//                                                    // cur_buf.copy_from_slice(&output);
//                 node.process(cur_buf.as_mut_slice()); // WHY IS THIS
//                 for (a, b) in zip(output.iter_mut(), cur_buf.iter()) {
//                     *a += b * (self.on_count as f32 / 100.);
//                 }
//             }
//         }
//         else if self.on_count > 0 {
//             self.on_count -= 1;
//             output.iter_mut().for_each( |x| *x = 0.);
//             for node in &mut self.inputs {
//                 let mut cur_buf = output.to_vec(); // pointless but
//                                                    // cur_buf.copy_from_slice(&output);
//                 node.process(cur_buf.as_mut_slice()); // WHY IS THIS
//                 for (a, b) in zip(output.iter_mut(), cur_buf.iter()) {
//                     *a += b * (self.on_count as f32 / 100.);
//                 }
//             }
//         }
//     }
// }
