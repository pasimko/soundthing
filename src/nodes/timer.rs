use super::{Node, SAMPLERATE, Message};
use std::sync::mpsc::channel;
use std::sync::mpsc::Sender;
use std::sync::mpsc::Receiver;

// I think it makes sense for there to be a high/low message sent here
// hmm
// Maybe NodeMessage should be its own enum, and each node can handle
// messages however they want?
// But like - it shouldn't be legal to send a signal to a node that will
// just ignore it...
// hmmm
#[derive(Clone)]
pub struct TimerParameters {
    pub time_low: u32,
    pub low_msg: Message,
    pub time_high: u32,
    pub high_msg: Message,
    pub name: String,
    pub tick: u32,
    pub sender: Sender<Message>,
}

pub struct TimerNode {
    params: TimerParameters,
    msg_receiver: Receiver<Message>,
    sinks: Vec<Sender<Message>>,
}

// TODO find a way to enforce low/high_msg sharing an enum value
impl TimerNode {
    pub fn new(n: &str, low_msg: Message, high_msg: Message) -> (Self, TimerParameters) {
        let (msg_sender, msg_receiver) = channel();
        let params = TimerParameters {
            time_low: 100,
            low_msg,
            time_high: 100,
            high_msg,
            name: n.to_owned(),
            tick: 0,
            sender: msg_sender,
        };
        let handler = params.clone();
        (Self {
            params,
            msg_receiver,
            sinks: Vec::new(),
        }, handler)
    }
    pub fn add_sink(&mut self, channel: Sender<Message>) {
        self.sinks.push(channel);
    }
}

impl Node for TimerNode {
    fn process(&mut self, output: &mut [f32]) {
        let msg = self.msg_receiver.try_recv();
        if let Ok(msg) = msg { match msg {
            Message::Interval(val) => self.params.time_low = val,
            Message::Duration(val) => self.params.time_high = val,
            _ => (),
        } };
        if self.params.tick == 0 { // output.len() as u32 {
            for handler in &self.sinks {
                let _ = handler.send(self.params.high_msg.clone());
            }
        }
        if self.params.tick == self.params.time_high { // >= (self.params.time_high + self.params.time_low) * (SAMPLERATE / output.len() ) as u32 {
            for handler in &self.sinks {
                let _ = handler.send(self.params.low_msg.clone());
            }
        }
        self.params.tick += 1; // output.len() as u32;
        self.params.tick %= self.params.time_high + self.params.time_low;
        // increment tick
        // self.params.tick %= (self.params.time_high + self.params.time_low) * SAMPLERATE as u32;
    }
}
