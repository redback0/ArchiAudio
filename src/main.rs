use std::sync::RwLock;
use wayland_client::{Connection, Dispatch, QueueHandle, event_created_child, globals::{GlobalListContents, registry_queue_init}, protocol::wl_registry};
use wayland_protocols_wlr::foreign_toplevel::v1::client::{zwlr_foreign_toplevel_manager_v1 as Manager, zwlr_foreign_toplevel_handle_v1 as Handle};

struct AppData;
struct HandleData {
    title: RwLock<String>,
}

impl Dispatch<wl_registry::WlRegistry, wayland_client::globals::GlobalListContents> for AppData {
    fn event(
        _state: &mut Self,
        _: &wl_registry::WlRegistry,
        _event: wl_registry::Event,
        _data: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<AppData>,
    ) {
    }
}

impl Dispatch<Manager::ZwlrForeignToplevelManagerV1, ()> for AppData {
    fn event(
        _state: &mut Self,
        _: &Manager::ZwlrForeignToplevelManagerV1,
        _event: <Manager::ZwlrForeignToplevelManagerV1 as wayland_client::Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }

    event_created_child!(AppData, Manager::ZwlrForeignToplevelManagerV1, [
        Manager::EVT_TOPLEVEL_OPCODE => (Handle::ZwlrForeignToplevelHandleV1, HandleData {title: String::from("").into()}),
    ]);
}

impl Dispatch<Handle::ZwlrForeignToplevelHandleV1, HandleData> for AppData {
    fn event(
        _state: &mut Self,
        _: &Handle::ZwlrForeignToplevelHandleV1,
        event: <Handle::ZwlrForeignToplevelHandleV1 as wayland_client::Proxy>::Event,
        handle_data: &HandleData,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let Handle::Event::Title { title } = event {
            let mut handle_title = handle_data.title.write().unwrap();
            *handle_title = title;
        }
        else if let Handle::Event::State { state } = event {
            println!("{}, {:?}", handle_data.title.read().unwrap(), state);
        }
    }
}

fn main() {
    let conn: Connection = Connection::connect_to_env().unwrap();
    let (globals, mut event_queue) = registry_queue_init::<AppData>(&conn).unwrap();

    let _toplevel_manager: Manager::ZwlrForeignToplevelManagerV1 = globals.bind(&event_queue.handle(), 3..=3, ()).unwrap();

    loop {
        let _ = event_queue.blocking_dispatch(&mut AppData);
    }
}
