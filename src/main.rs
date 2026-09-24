use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use iced;

use serde::{Deserialize, Serialize};

mod pactl_action;
mod wayland;

use crate::pactl_action::PactlActionGenerator;
use crate::wayland::{HandleWrapper, WLIcedMessage, Wayland};

pub trait Action: Send + Sync {
    fn trigger(&mut self) -> Result<(), String>;
    fn get_generator(&self) -> GeneratorIndex;
    fn view<'a>(
        &self,
        self_lock: Arc<RwLock<dyn Action>>,
        generators: &'a Vec<Arc<dyn ActionGenerator<dyn Action>>>,
    ) -> iced::Element<'a, IcedMessage>;
    fn update(
        &mut self,
        new_data: ActionDataID,
        generators: &Vec<Arc<dyn ActionGenerator<dyn Action>>>,
    );
}

pub trait ActionGenerator<T>: std::fmt::Display
where
    T: Action + ?Sized,
{
    fn build_action(self: Arc<Self>, gener_idx: GeneratorIndex) -> Result<Arc<RwLock<T>>, String>;
    fn build_action_by_desc(
        self: Arc<Self>,
        gener_idx: GeneratorIndex,
        desc: String,
    ) -> Result<Arc<RwLock<T>>, String>;
    fn get_action_name(&self) -> String;
    fn as_any(&self) -> &dyn std::any::Any;
}

#[derive(Clone)]
pub struct GeneratorIndex {
    index: usize,
    name: String,
}

#[derive(Clone)]
pub struct ActionDataID {
    _option_idx: usize,
    id: u64,
    description: String,
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct EventDesc {
    provider_desc: String,
    event_desc: String,
}

struct ActionDesc {
    gen_desc: String,
    action_desc: String,
}

#[derive(Deserialize, Serialize)]
struct SavedActionDesc {
    provider_desc: String,
    event_desc: String,
    gen_desc: String,
    action_desc: String,
}

struct IcedState {
    generators: Vec<Arc<dyn ActionGenerator<dyn Action>>>,
    generators_state: iced::widget::combo_box::State<GeneratorIndex>,
    wayland: Wayland,
    saved_actions: BTreeMap<EventDesc, ActionDesc>,
}

#[derive(Clone)]
pub enum IcedMessage {
    NewWlEvent(HandleWrapper),
    UpdateAction(Arc<RwLock<dyn Action>>, ActionDataID),
    Wayland(WLIcedMessage),
    Noop,
}

impl std::fmt::Display for GeneratorIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name)
    }
}

impl std::fmt::Display for ActionDataID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.description)
    }
}

fn save_actions(actions: BTreeMap<EventDesc, ActionDesc>) -> Result<(), String> {
    let mut path = match std::env::home_dir() {
        Some(v) => v,
        None => return Err(format!("Could not find home dir")),
    };
    path.push(".local/share/ArchiAudio/saved_actions.json");

    let actions_json = match serde_json::to_vec(
        &actions
            .iter()
            .map(|(event, action)| SavedActionDesc {
                provider_desc: event.provider_desc.clone(),
                event_desc: event.event_desc.clone(),
                gen_desc: action.gen_desc.clone(),
                action_desc: action.action_desc.clone(),
            })
            .collect::<Vec<_>>(),
    ) {
        Ok(v) => v,
        Err(err) => return Err(format!("Failed to serialize: {}", err)),
    };

    match std::fs::write(path, actions_json) {
        Ok(_) => Ok(()),
        Err(err) => Err(format!("Failed to write file: {}", err)),
    }
}

fn load_saved_actions() -> BTreeMap<EventDesc, ActionDesc> {
    let mut path = match std::env::home_dir() {
        Some(v) => v,
        None => return BTreeMap::new(),
    };
    path.push(".local/share/ArchiAudio/saved_actions.json");
    let saved_actions_raw = {
        match std::fs::read(path) {
            Ok(raw) => raw,
            Err(_err) => vec![],
        }
    };

    let saved_actions_vec: Vec<SavedActionDesc> =
        serde_json::from_slice(&saved_actions_raw).unwrap_or_default();

    BTreeMap::from_iter(saved_actions_vec.iter().map(|saved_action| {
        (
            EventDesc {
                provider_desc: saved_action.provider_desc.clone(),
                event_desc: saved_action.event_desc.clone(),
            },
            ActionDesc {
                gen_desc: saved_action.gen_desc.clone(),
                action_desc: saved_action.action_desc.clone(),
            },
        )
    }))
}

impl Default for IcedState {
    fn default() -> Self {
        let geners: Vec<Arc<dyn ActionGenerator<dyn Action>>> =
            vec![Arc::new(PactlActionGenerator::new())];
        let s = Self {
            generators_state: iced::widget::combo_box::State::new(
                geners
                    .iter()
                    .enumerate()
                    .map(|(index, gener)| GeneratorIndex {
                        index,
                        name: format!("{}", gener),
                    })
                    .collect(),
            ),
            generators: geners,
            wayland: Wayland::default(),
            saved_actions: load_saved_actions(),
        };

        s
    }
}

impl IcedState {
    fn update(&mut self, message: IcedMessage) -> iced::Task<IcedMessage> {
        match message {
            IcedMessage::NewWlEvent(handle_lock) => {
                let event_desc = EventDesc {
                    provider_desc: String::new(), // TODO: actually specify what provider this is
                    event_desc: handle_lock.read().unwrap().get_title(),
                };

                //println!(
                //    "::{}::, ::{}::",
                //    event_desc.provider_desc, event_desc.event_desc
                //);

                let action_desc_opt = self.saved_actions.get(&event_desc);

                if action_desc_opt.is_some() {
                    let action_desc = action_desc_opt.unwrap();

                    for (index, gener) in self.generators.iter().enumerate() {
                        if gener.get_action_name() == action_desc.gen_desc {
                            match gener.clone().build_action_by_desc(
                                GeneratorIndex {
                                    index,
                                    name: format!("{}", gener),
                                },
                                action_desc.action_desc.clone(),
                            ) {
                                Ok(new_action) => {
                                    wayland::Wayland::add_action(handle_lock.clone(), new_action)
                                }
                                Err(str) => println!("Failed to create action: {}", str),
                            }
                        }
                    }
                }

                iced::Task::none()
            }
            IcedMessage::UpdateAction(action_lock, data) => {
                action_lock.write().unwrap().update(data, &self.generators);
                iced::Task::none()
            }
            IcedMessage::Wayland(v) => self.wayland.update(self, v),
            IcedMessage::Noop => iced::Task::none(),
        }
    }

    fn view(&self) -> iced::Element<'_, IcedMessage> {
        self.wayland.view(self)
    }

    fn subscription(&self) -> iced::Subscription<IcedMessage> {
        iced::Subscription::run(wayland::client)
    }
}

fn main() -> iced::Result {
    iced::application(IcedState::default, IcedState::update, IcedState::view)
        .subscription(IcedState::subscription)
        .run()
}
