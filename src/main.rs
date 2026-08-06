
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

impl IcedState {
    fn update(&mut self, message: IcedMessage) {
        match message {
            IcedMessage::Wayland(v) => self.wayland.update(v),
        }
    }
    
    fn view(&self) -> iced::Element<'_, IcedMessage> {
        self.wayland.view(self)
    }
}

fn main() -> iced::Result {
    iced::application(IcedState::default, IcedState::update, IcedState::view).run()
}
