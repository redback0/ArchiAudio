use std::{sync::{Arc, RwLock}, vec};

use wayland_client::{Connection, Dispatch, Proxy};
use wayland_protocols_wlr::foreign_toplevel::v1::client::{
    zwlr_foreign_toplevel_manager_v1 as top_level_manager,
    zwlr_foreign_toplevel_handle_v1 as top_level_handle
};
use iced;

use crate::{Action, GeneratorIndex, IcedMessage, IcedState};

pub struct Wayland {
    _tc: std::sync::mpsc::Sender<ChannelMessage>,
    handles_lock: HandleVec,
}

struct WaylandInternal {
    handles_lock: HandleVec,
}

pub struct TLHandleActions {
    title: String,
    id: String,
    curr_active: bool,
    active_actions: Vec<Arc<RwLock<dyn Action>>>,
    deactive_actions: Vec<Arc<RwLock<dyn Action>>>,
}

pub type HandleWrapper = Arc<RwLock<TLHandleActions>>;
type HandleVec = Arc<RwLock<Vec<HandleWrapper>>>;

enum ChannelMessage {
    _CONTINUE,
    _EXIT,
}

#[derive(Clone)]
pub enum WLIcedMessage {
    ChangeActiveActionType(
        GeneratorIndex,
        HandleWrapper,
        Option<Arc<RwLock<dyn Action>>>
    ),
    RemoveAction(
        HandleWrapper,
        Arc<RwLock<dyn Action>>,
    ),
}

impl Wayland {
    pub fn view<'a>(&'a self, state: &'a IcedState) -> iced::Element<'a, IcedMessage>{
        let mut column = iced::widget::column![];
        
        let handles = self.handles_lock.read().unwrap();

        for handle_lock in handles.iter() {
            let handle = handle_lock.read().unwrap();
            let mut row = iced::widget::row![];

            row = row.push(iced::widget::text!("{}", handle.title).width(500));
            let mut act_col = iced::widget::column![];
            for action_lock in handle.active_actions.clone() {
                let move_handle_lock = handle_lock.clone();
                let move_action_lock = action_lock.clone();
                let action = action_lock.read().unwrap();
                let act_row = iced::widget::row![
                    iced::widget::combo_box(
                        &state.generators_state,
                        "Pick an action",
                        Some(&action.get_generator()),
                        move |generator| IcedMessage::Wayland(
                            WLIcedMessage::ChangeActiveActionType(
                                generator,
                                move_handle_lock.clone(),
                                Some(move_action_lock.clone())
                            )
                        )
                    ).width(170),
                    iced::widget::button("X").on_press(IcedMessage::Wayland(WLIcedMessage::RemoveAction(handle_lock.clone(), action_lock.clone()))),
                    action.view(action_lock.clone(), &state.generators)
                ];
                act_col = act_col.push(act_row);
            }
            let move_action_lock = handle_lock.clone();
            act_col = act_col.push(iced::widget::combo_box(
                &state.generators_state,
                "Pick an action",
                None,
                move |generator| IcedMessage::Wayland(
                    WLIcedMessage::ChangeActiveActionType(
                        generator,
                        move_action_lock.clone(),
                        None
                    )
                )
            ).width(200));
            row = row.push(act_col);
            column = column.push(row);
        }

        column.into()
    }

    pub fn update(&self, state: &IcedState, message: WLIcedMessage) {
        match message {
            WLIcedMessage::ChangeActiveActionType(gener_idx, handle_lock, action_lock_option) => {
                let gener = state.generators[gener_idx.index].clone();
                match action_lock_option {
                    Some(action_lock) => {
                        let action = action_lock.read().unwrap();

                        if gener_idx.index == action.get_generator().index {
                            return
                        }

                        let mut handle = handle_lock.write().unwrap();

                        match handle.active_actions
                        .iter()
                        .position(|old_action|
                            Arc::ptr_eq(old_action, &action_lock)
                        ) {
                            Some(index) => handle.active_actions[index] = gener.build_action(gener_idx).unwrap(),
                            _ => {}
                        }
                    }
                    None => {
                        let mut handle = handle_lock.write().unwrap();

                        handle.active_actions.push(gener.build_action(gener_idx).unwrap());
                    }
                }
            }
            WLIcedMessage::RemoveAction(handle_lock, action_lock) => {
                let mut handle = handle_lock.write().unwrap();

                match handle.active_actions
                .iter()
                .position(|old_action|
                    Arc::ptr_eq(old_action, &action_lock)
                ) {
                    Some(index) => _ = handle.active_actions.remove(index),
                    _ => {}
                }
            }
        }
    }
}

impl Default for Wayland {
    fn default() -> Self {
        let handles_lock = HandleVec::new(std::sync::RwLock::new(vec![]));
        let conn: Connection = Connection::connect_to_env().unwrap();
        let (globals, mut event_queue) = wayland_client::globals::registry_queue_init::<WaylandInternal>(&conn).unwrap();

        let _toplevel_manager: top_level_manager::ZwlrForeignToplevelManagerV1 = globals.bind(&event_queue.handle(), 3..=3, handles_lock.clone()).unwrap();

        let mut wl = WaylandInternal{handles_lock: handles_lock.clone()};
        event_queue.roundtrip(&mut wl).unwrap();

        let (tc, rc) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            loop {
                let read_lock = event_queue.prepare_read().unwrap();

                match read_lock.read() {
                    Ok(_) => {
                        let _ = event_queue.dispatch_pending(&mut wl);
                    }
                    Err(_) => {}
                }
                match rc.try_recv() {
                    Ok(ChannelMessage::_EXIT) | Err(std::sync::mpsc::TryRecvError::Disconnected)
                        => return,
                    Ok(_) | Err(_) => {}, 
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        });
        Self{_tc: tc, handles_lock}
    }
}

impl Dispatch<wayland_client::protocol::wl_registry::WlRegistry, wayland_client::globals::GlobalListContents> for WaylandInternal {
    fn event(
        _: &mut Self,
        _: &wayland_client::protocol::wl_registry::WlRegistry,
        _event: wayland_client::protocol::wl_registry::Event,
        _data: &wayland_client::globals::GlobalListContents,
        _: &wayland_client::Connection,
        _: &wayland_client::QueueHandle<WaylandInternal>,
    ) {
        // purposely empty
    }
}

impl Dispatch<top_level_manager::ZwlrForeignToplevelManagerV1, HandleVec> for WaylandInternal {
    fn event(
        _: &mut Self,
        _: &top_level_manager::ZwlrForeignToplevelManagerV1,
        event: <top_level_manager::ZwlrForeignToplevelManagerV1 as wayland_client::Proxy>::Event,
        handles_lock: &HandleVec,
        _: &wayland_client::Connection,
        _: &wayland_client::QueueHandle<Self>,
    ) {
        match event {
            top_level_manager::Event::Toplevel{toplevel} => {
                let user_data: Option<&HandleWrapper> = toplevel.data();

                match user_data {
                    Some(handle_data) => {
                        handles_lock.write().unwrap().push(handle_data.clone());
                    },
                    None => panic!("Failed to get user data from toplevel handle"),
                }
            }
            _ => {}
        }
    }

    wayland_client::event_created_child!(WaylandInternal, top_level_manager::ZwlrForeignToplevelManagerV1, [
        top_level_manager::EVT_TOPLEVEL_OPCODE => (top_level_handle::ZwlrForeignToplevelHandleV1,
            std::sync::Arc::new(TLHandleActions {
                title: String::new(),
                id: String::new(),
                curr_active: false,
                active_actions: vec!(),
                deactive_actions: vec!(),
            }.into())),
    ]);
}

impl Dispatch<top_level_handle::ZwlrForeignToplevelHandleV1, HandleWrapper> for WaylandInternal {
    fn event(
        wl_state: &mut Self,
        _: &top_level_handle::ZwlrForeignToplevelHandleV1,
        event: <top_level_handle::ZwlrForeignToplevelHandleV1 as wayland_client::Proxy>::Event,
        handle_data_lock: &HandleWrapper,
        _: &wayland_client::Connection,
        _: &wayland_client::QueueHandle<Self>,
    ) {

        match event {
            top_level_handle::Event::Title { title } => {
                let mut handle_data = handle_data_lock.write().unwrap();
                handle_data.title = title;
            },
            top_level_handle::Event::AppId { app_id } => {
                let mut handle_data = handle_data_lock.write().unwrap();
                handle_data.id = app_id;
            },
            top_level_handle::Event::State { state } => {
                let mut handle_data = handle_data_lock.write().unwrap();

                if !handle_data.curr_active && state.contains(&2) { // unable to use State enum directly
                    handle_data.curr_active = true;
                    // println!("Active: {}", handle_data.title);

                    for action_lock in &mut handle_data.active_actions {
                        let mut action = action_lock.write().unwrap();
                        match action.trigger() {
                            Ok(_) => {},
                            Err(e) => println!("Action failed: {}", e),
                        }
                    }
                } else if handle_data.curr_active && !state.contains(&2) {
                    handle_data.curr_active = false;
                    // println!("Deactive: {}", handle_data.title);

                    for action_lock in &mut handle_data.deactive_actions {
                        let mut action = action_lock.write().unwrap();
                        match action.trigger() {
                            Ok(_) => {},
                            Err(e) => println!("Action failed: {}", e),
                        }
                    }
                }
            },
            top_level_handle::Event::Closed => {
                let mut handles = wl_state.handles_lock.write().unwrap();

                match handles.iter().position(
                    |p| Arc::ptr_eq(handle_data_lock, p)
                ) {
                    Some(index) => _ = handles.remove(index),
                    _ => {}
                }
            }
            _ => {}
        }
    }
}
