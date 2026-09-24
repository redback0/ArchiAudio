use std::{
    sync::{Arc, RwLock},
    vec,
};

use iced;
use iced::futures::SinkExt;
use wayland_client::{Connection, Dispatch, Proxy};
use wayland_protocols_wlr::foreign_toplevel::v1::client::{
    zwlr_foreign_toplevel_handle_v1 as top_level_handle,
    zwlr_foreign_toplevel_manager_v1 as top_level_manager,
};

use crate::{Action, GeneratorIndex, IcedMessage, IcedState};

pub struct Wayland {
    handles_lock: HandleVec,
}

struct WaylandInternal {
    handles_lock: HandleVec,
    iced_sender: iced::futures::channel::mpsc::Sender<IcedMessage>,
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

#[derive(Clone)]
pub enum WLIcedMessage {
    ChangeActiveActionType(
        GeneratorIndex,
        HandleWrapper,
        Option<Arc<RwLock<dyn Action>>>,
    ),
    RemoveAction(HandleWrapper, Arc<RwLock<dyn Action>>),
    AddAction(HandleWrapper, Arc<RwLock<dyn Action>>),
    WLSetup(WLSetup),
}

#[derive(Clone)]
pub enum WLSetup {
    Sender(iced::futures::channel::mpsc::Sender<WLSetup>),
    HandlesLock(HandleVec),
}

impl Wayland {
    pub fn view<'a>(&'a self, state: &'a IcedState) -> iced::Element<'a, IcedMessage> {
        let mut column = iced::widget::column![].width(iced::Length::Fill);

        let handles = self.handles_lock.read().unwrap();

        for handle_lock in handles.iter() {
            let handle = handle_lock.read().unwrap();
            let mut row = iced::widget::row![].padding(2);

            row = row.push(iced::widget::text!("{}", handle.title).width(iced::Length::Fill));
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
                    )
                    .width(170),
                    iced::widget::button("X").on_press(IcedMessage::Wayland(
                        WLIcedMessage::RemoveAction(handle_lock.clone(), action_lock.clone())
                    )),
                    action.view(action_lock.clone(), handle_lock.clone(), &state.generators)
                ];
                act_col = act_col.push(act_row);
            }
            let move_action_lock = handle_lock.clone();
            act_col = act_col.push(iced::widget::row![
                iced::widget::combo_box(
                    &state.generators_state,
                    "Pick an action",
                    None,
                    move |generator| {
                        IcedMessage::Wayland(WLIcedMessage::ChangeActiveActionType(
                            generator,
                            move_action_lock.clone(),
                            None,
                        ))
                    },
                )
                .width(200),
                iced::widget::text!("").width(200)
            ]);
            row = row.push(act_col);
            column = column.push(row);
        }

        iced::widget::scrollable(column).into()
    }

    pub fn update(&self, state: &IcedState, message: WLIcedMessage) -> iced::Task<IcedMessage> {
        match message {
            WLIcedMessage::ChangeActiveActionType(gener_idx, handle_lock, action_lock_option) => {
                let gener = state.generators[gener_idx.index].clone();
                match action_lock_option {
                    Some(action_lock) => {
                        let action = action_lock.read().unwrap();

                        if gener_idx.index == action.get_generator().index {
                            return iced::Task::none();
                        }

                        let mut handle = handle_lock.write().unwrap();

                        match handle
                            .active_actions
                            .iter()
                            .position(|old_action| Arc::ptr_eq(old_action, &action_lock))
                        {
                            Some(index) => {
                                handle.active_actions[index] =
                                    gener.build_action(gener_idx).unwrap()
                            }
                            _ => {}
                        }
                    }
                    None => {
                        let mut handle = handle_lock.write().unwrap();

                        handle
                            .active_actions
                            .push(gener.build_action(gener_idx).unwrap());
                    }
                }
                iced::Task::none()
            }
            WLIcedMessage::AddAction(handle_lock, action_lock) => {
                let mut handle = handle_lock.write().unwrap();

                handle.active_actions.push(action_lock);
                iced::Task::none()
            }
            WLIcedMessage::RemoveAction(handle_lock, action_lock) => {
                let mut handle = handle_lock.write().unwrap();

                match handle
                    .active_actions
                    .iter()
                    .position(|old_action| Arc::ptr_eq(old_action, &action_lock))
                {
                    Some(index) => _ = handle.active_actions.remove(index),
                    _ => {}
                }
                iced::Task::none()
            }
            WLIcedMessage::WLSetup(setup) => match setup {
                WLSetup::Sender(mut tc) => {
                    tc.try_send(WLSetup::HandlesLock(self.handles_lock.clone()))
                        .unwrap();
                    iced::Task::none()
                }
                WLSetup::HandlesLock(_) => panic!(),
            },
        }
    }

    pub fn add_action(handle_lock: HandleWrapper, action_lock: Arc<RwLock<dyn Action + 'static>>) {
        let mut handle = handle_lock.write().unwrap();

        handle.active_actions.push(action_lock);
    }
}

pub fn client() -> impl futures_core::stream::Stream<Item = IcedMessage> {
    iced::stream::channel(100, async |mut output| {
        let (tc, mut rc) = iced::futures::channel::mpsc::channel(16);
        output
            .send(IcedMessage::Wayland(WLIcedMessage::WLSetup(
                WLSetup::Sender(tc),
            )))
            .await
            .unwrap();

        let handles_lock = match rc.recv().await {
            Ok(WLSetup::HandlesLock(v)) => v,
            _ => panic!(),
        };

        let conn: Connection = Connection::connect_to_env().unwrap();
        let (globals, mut event_queue) =
            wayland_client::globals::registry_queue_init::<WaylandInternal>(&conn).unwrap();

        let _toplevel_manager: top_level_manager::ZwlrForeignToplevelManagerV1 = globals
            .bind(&event_queue.handle(), 3..=3, handles_lock.clone())
            .unwrap();

        let mut wl = WaylandInternal {
            handles_lock: handles_lock.clone(),
            iced_sender: output,
        };
        event_queue.roundtrip(&mut wl).unwrap();

        loop {
            let read_lock = event_queue.prepare_read().unwrap();

            match read_lock.read() {
                Ok(_) => {
                    let _ = event_queue.dispatch_pending(&mut wl);
                }
                Err(_) => {}
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    })
}

impl Default for Wayland {
    fn default() -> Self {
        let handles_lock = HandleVec::new(std::sync::RwLock::new(vec![]));
        Self { handles_lock }
    }
}

impl TLHandleActions {
    pub fn get_title(&self) -> String {
        self.title.clone()
    }
}

impl
    Dispatch<
        wayland_client::protocol::wl_registry::WlRegistry,
        wayland_client::globals::GlobalListContents,
    > for WaylandInternal
{
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
            top_level_manager::Event::Toplevel { toplevel } => {
                let user_data: Option<&HandleWrapper> = toplevel.data();

                match user_data {
                    Some(handle_data) => {
                        handles_lock.write().unwrap().push(handle_data.clone());
                    }
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
                wl_state
                    .iced_sender
                    .try_send(IcedMessage::NewWlEvent(handle_data_lock.clone()))
                    .ok();
            }
            top_level_handle::Event::AppId { app_id } => {
                let mut handle_data = handle_data_lock.write().unwrap();
                handle_data.id = app_id;
            }
            top_level_handle::Event::State { state } => {
                let mut handle_data = handle_data_lock.write().unwrap();

                if !handle_data.curr_active && state.contains(&2) {
                    // unable to use State enum directly
                    handle_data.curr_active = true;
                    // println!("Active: {}", handle_data.title);

                    for action_lock in &mut handle_data.active_actions {
                        let mut action = action_lock.write().unwrap();
                        match action.trigger() {
                            Ok(_) => {}
                            Err(e) => println!("Action failed: {}", e),
                        }
                    }
                } else if handle_data.curr_active && !state.contains(&2) {
                    handle_data.curr_active = false;
                    // println!("Deactive: {}", handle_data.title);

                    for action_lock in &mut handle_data.deactive_actions {
                        let mut action = action_lock.write().unwrap();
                        match action.trigger() {
                            Ok(_) => {}
                            Err(e) => println!("Action failed: {}", e),
                        }
                    }
                }
            }
            top_level_handle::Event::Closed => {
                let mut handles = wl_state.handles_lock.write().unwrap();

                match handles
                    .iter()
                    .position(|p| Arc::ptr_eq(handle_data_lock, p))
                {
                    Some(index) => _ = handles.remove(index),
                    _ => {}
                }
                wl_state.iced_sender.try_send(IcedMessage::Noop).ok();
            }
            _ => {}
        }
    }
}
