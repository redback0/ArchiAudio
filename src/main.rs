
use iced;

mod wayland;
mod pactl_action;

use crate::wayland::{Wayland, WLIcedMessage};
use crate::pactl_action::PactlActionGenerator;


pub trait Action: Send + Sync {
    fn trigger(&mut self) -> Result<(), String>;
}

pub trait ActionGenerator<T>: where T: Action + ?Sized {
    fn build_action(&self) -> Result<Box<T>, String>;
    fn get_action_name(&self) -> String;
}

struct IcedState {
    selections: iced::widget::combo_box::State<IcedSelectBox>,
    selection: Option<IcedSelectBox>,
    wayland: Wayland,
}

#[derive(Debug, Clone)]
pub enum IcedMessage {
    SELECTBOX(IcedSelectBox),
    WAYLAND(WLIcedMessage),
}

#[derive(Debug, Clone)]
enum IcedSelectBox {
    A,
    B,
    C,
}

impl Default for IcedState {
    fn default() -> Self {
        let mut s = Self {
            selections: iced::widget::combo_box::State::<IcedSelectBox>::default(),
            selection: None,
            wayland: Wayland::default(),
        };
        s.selections.push(IcedSelectBox::A);
        s.selections.push(IcedSelectBox::B);
        s.selections.push(IcedSelectBox::C);

        s
    }
}

impl std::fmt::Display for IcedSelectBox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::A => "a thing",
            Self::B => "b nother thing",
            Self::C => "c gain",
        })
    }
}

fn iced_update(state: &mut IcedState, message: IcedMessage) {
    match message {
        IcedMessage::SELECTBOX(v) => {
            println!("{}", v);
            state.selection = Some(v);
        }
    }
}

fn iced_view(state: &IcedState) -> iced::Element<'_, IcedMessage> {
    println!("{:?}", state.selections);
    iced::widget::combo_box(
        &state.selections,
        "select action",
        state.selection.as_ref(),
        IcedMessage::SELECTBOX
    ).into()
}

fn main() -> iced::Result {
    let mut generators: Vec<Box<dyn ActionGenerator<dyn Action>>> = vec!();
    generators.push(Box::new(PactlActionGenerator::new()));

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
