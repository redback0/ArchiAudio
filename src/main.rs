use std::sync::{Arc, RwLock};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle, event_created_child, globals::{GlobalListContents, registry_queue_init}, protocol::wl_registry};
use wayland_protocols_wlr::foreign_toplevel::v1::client::{zwlr_foreign_toplevel_manager_v1 as Manager, zwlr_foreign_toplevel_handle_v1 as Handle};

struct AppData;
struct HandleData {
    title: String,
    // do_swap: bool,
    // pa_source: String,
}

type HandleWrapper = Arc<RwLock<HandleData>>;
type HandleVec = Arc<RwLock<Vec<HandleWrapper>>>;

impl Dispatch<wl_registry::WlRegistry, wayland_client::globals::GlobalListContents> for AppData {
    fn event(
        _: &mut Self,
        _: &wl_registry::WlRegistry,
        _event: wl_registry::Event,
        _data: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<AppData>,
    ) {
    }
}

impl Dispatch<Manager::ZwlrForeignToplevelManagerV1, HandleVec> for AppData {
    fn event(
        _: &mut Self,
        _: &Manager::ZwlrForeignToplevelManagerV1,
        event: <Manager::ZwlrForeignToplevelManagerV1 as wayland_client::Proxy>::Event,
        data: &HandleVec,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let Manager::Event::Toplevel { toplevel } = event {
            let user_data: Option<&Arc<RwLock<HandleData>>> = toplevel.data();

            match user_data {
                Some(handle_data) => {
                    data.write().unwrap().push(handle_data.clone());
                },
                None => println!("you're a failure"),
            }
        }
    }

    event_created_child!(AppData, Manager::ZwlrForeignToplevelManagerV1, [
        Manager::EVT_TOPLEVEL_OPCODE => (Handle::ZwlrForeignToplevelHandleV1,
            Arc::new(HandleData {
                title: String::new(),
                // do_swap: false,
                // pa_source: String::new(),
            }.into())),
    ]);
}

impl Dispatch<Handle::ZwlrForeignToplevelHandleV1, HandleWrapper> for AppData {
    fn event(
        _: &mut Self,
        _: &Handle::ZwlrForeignToplevelHandleV1,
        event: <Handle::ZwlrForeignToplevelHandleV1 as wayland_client::Proxy>::Event,
        handle_data_lock: &HandleWrapper,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let Handle::Event::Title { title } = event {
            let mut handle_data = handle_data_lock.write().unwrap();
            handle_data.title = title;
        }
        else if let Handle::Event::State { state } = event {
            if state.contains(&2) {
                // println!("Active: {}", handle_data_lock.read().unwrap().title);
                // handle audio swap here
            }
        }
    }
}

fn main() {
    let handles = HandleVec::new(RwLock::new(vec![]));

    let conn: Connection = Connection::connect_to_env().unwrap();
    let (globals, mut event_queue) = registry_queue_init::<AppData>(&conn).unwrap();

    let _toplevel_manager: Manager::ZwlrForeignToplevelManagerV1 = globals.bind(&event_queue.handle(), 3..=3, handles.clone()).unwrap();

    let _ = event_queue.roundtrip(&mut AppData);

    // init connections

    handles.read().unwrap().iter().for_each(|handle| {
        println!("{}", handle.read().unwrap().title)
    });

    std::thread::spawn(move || {
        loop {
            let _ = event_queue.blocking_dispatch(&mut AppData);
        }
    });

    let stdin = std::io::stdin();
    let mut buf = String::new();
    loop {
        let _ = stdin.read_line(&mut buf);
    }
}
