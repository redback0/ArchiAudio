
use std::sync::{Arc, RwLock};

use iced;

mod wayland;
mod pactl_action;

use crate::wayland::{Wayland, WLIcedMessage};
use crate::pactl_action::PactlActionGenerator;


pub trait Action: Send + Sync {
    fn trigger(&mut self) -> Result<(), String>;
    fn action_type(&self) -> String;
    fn get_generator(&self) -> Arc<dyn ActionGenerator<dyn Action>>;
}

pub trait ActionGenerator<T>: std::fmt::Display + Send + Sync where T: Action + ?Sized {
    fn build_action(self: Arc<Self>) -> Result<Arc<RwLock<T>>, String>;
    fn get_action_name(&self) -> String;
}

struct IcedState {
    generators: iced::widget::combo_box::State<Arc<dyn ActionGenerator<dyn Action>>>,
    wayland: Wayland,
}

#[derive(Clone)]
pub enum IcedMessage {
    Wayland(WLIcedMessage),
}

impl Default for IcedState {
    fn default() -> Self {
        let s = Self {
            generators: iced::widget::combo_box::State::new(vec![
                Arc::new(PactlActionGenerator::new()),
            ]),
            wayland: Wayland::default(),
        };
        s
    }
}

fn iced_update(state: &mut IcedState, message: IcedMessage) {
    match message {
        IcedMessage::Wayland(v) => state.wayland.update(v),
    }
}

fn iced_view(state: &IcedState) -> iced::Element<'_, IcedMessage> {
    state.wayland.view(state)
}

fn main() -> iced::Result {

    iced::run(iced_update, iced_view)
}


// fn create_actions(handles_lock: &HandleVec, generators: &Vec<Box<dyn ActionGenerator<dyn Action>>>) {
//     let handles = handles_lock.read().unwrap();
//     let stdin = std::io::stdin();
//     let mut buf = String::new();

//     loop {
//         handles.iter().enumerate().for_each(|(i, handle)| {
//             println!("[{}] {}", i, handle.read().unwrap().title)
//         });
//         println!("\nWhich window would you like to have swap? (leave empty to skip)");
//         buf.clear();
//         if stdin.read_line(&mut buf).unwrap_or(0) <= 1 { break };

//         let handle_index = buf.trim().parse::<usize>().unwrap_or(usize::MAX);

//         if handle_index >= handles.len() {
//             println!("invalid index");
//             continue;
//         }

//         println!("\n");

//         generators.iter().enumerate().for_each(|(i, generator)| {
//             println!("[{}] {}", i, generator.get_action_name())
//         });

//         buf.clear();
//         if stdin.read_line(&mut buf).unwrap_or(0) <= 1 {
//             println!("invalid index");
//             continue;
//         };

//         let generator_index = buf.trim().parse::<usize>().unwrap_or(usize::MAX);

//         if generator_index >= generators.len() {
//             println!("invalid index");
//             continue;
//         }

//         let generator = generators[generator_index].as_ref();

//         let comm = generator.build_action();

//         match comm {
//             Ok(v) => handles[handle_index].write().unwrap().active_actions.push(v),
//             Err(e) => println!("Failed to set action: {}", e),
//         }
//     }
// }
