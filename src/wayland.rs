use std::vec;

use Box;
use wayland_client::{Connection, Dispatch, Proxy};
use wayland_protocols_wlr::foreign_toplevel::v1::client::{
    zwlr_foreign_toplevel_manager_v1 as top_level_manager,
    zwlr_foreign_toplevel_handle_v1 as top_level_handle
};
use iced;

use crate::{Action, IcedMessage};

pub struct Wayland {
    _tc: std::sync::mpsc::Sender<ChannelMessage>,
    _handles_lock: HandleVec,
}

struct AppData;
struct TLHandleActions {
    title: String,
    id: String,
    curr_active: bool,
    active_actions: Vec<Box<dyn Action>>,
    deactive_actions: Vec<Box<dyn Action>>,
}

type HandleWrapper = std::sync::Arc<std::sync::RwLock<TLHandleActions>>;
type HandleVec = std::sync::Arc<std::sync::RwLock<Vec<HandleWrapper>>>;

enum ChannelMessage {
    _CONTINUE,
    _EXIT,
}

#[derive(Debug, Clone)]
pub enum WLIcedMessage {
}

impl Wayland {
    pub fn view(&self) -> iced::Element<'_, IcedMessage>{
        iced::widget::text("hello").into()
    }

    pub fn update(&self, message: WLIcedMessage) {

    }
}

impl Default for Wayland {
    fn default() -> Self {
        let handles_lock = HandleVec::new(std::sync::RwLock::new(vec![]));
        let conn: Connection = Connection::connect_to_env().unwrap();
        let (globals, mut event_queue) = wayland_client::globals::registry_queue_init::<AppData>(&conn).unwrap();

        let _toplevel_manager: top_level_manager::ZwlrForeignToplevelManagerV1 = globals.bind(&event_queue.handle(), 3..=3, handles_lock.clone()).unwrap();

        event_queue.roundtrip(&mut AppData).unwrap();

        let (tc, rc) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            loop {
                let read_lock = event_queue.prepare_read().unwrap();

                match read_lock.read() {
                    Ok(_) => {
                        let _ = event_queue.dispatch_pending(&mut AppData);
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
        Self{_tc: tc, _handles_lock: handles_lock}
    }
}

impl Dispatch<wayland_client::protocol::wl_registry::WlRegistry, wayland_client::globals::GlobalListContents> for AppData {
    fn event(
        _: &mut Self,
        _: &wayland_client::protocol::wl_registry::WlRegistry,
        _event: wayland_client::protocol::wl_registry::Event,
        _data: &wayland_client::globals::GlobalListContents,
        _: &wayland_client::Connection,
        _: &wayland_client::QueueHandle<AppData>,
    ) {
        // purposely empty
    }
}

impl Dispatch<top_level_manager::ZwlrForeignToplevelManagerV1, HandleVec> for AppData {
    fn event(
        _: &mut Self,
        _: &top_level_manager::ZwlrForeignToplevelManagerV1,
        event: <top_level_manager::ZwlrForeignToplevelManagerV1 as wayland_client::Proxy>::Event,
        data: &HandleVec,
        _: &wayland_client::Connection,
        _: &wayland_client::QueueHandle<Self>,
    ) {
        if let top_level_manager::Event::Toplevel { toplevel } = event {
            let user_data: Option<&HandleWrapper> = toplevel.data();

            match user_data {
                Some(handle_data) => {
                    data.write().unwrap().push(handle_data.clone());
                },
                None => panic!("Failed to get user data from toplevel handle"),
            }
        }
    }

    wayland_client::event_created_child!(AppData, top_level_manager::ZwlrForeignToplevelManagerV1, [
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

impl Dispatch<top_level_handle::ZwlrForeignToplevelHandleV1, HandleWrapper> for AppData {
    fn event(
        _: &mut Self,
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
                    println!("Active: {}", handle_data.title);

                    for action in &mut handle_data.active_actions {
                        match action.trigger() {
                            Ok(_) => {},
                            Err(e) => println!("Action failed: {}", e),
                        }
                    }
                } else if handle_data.curr_active && !state.contains(&2) {
                    handle_data.curr_active = false;
                    println!("Deactive: {}", handle_data.title);

                    for action in &mut handle_data.deactive_actions {
                        match action.trigger() {
                            Ok(_) => {},
                            Err(e) => println!("Action failed: {}", e),
                        }
                    }
                }
            },
            _ => {}
        }
    }
}
