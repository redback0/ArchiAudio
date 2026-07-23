use std::sync::{Arc, RwLock};
use serde::{Deserialize, Serialize};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle, event_created_child, globals::{GlobalListContents, registry_queue_init}, protocol::wl_registry};
use wayland_protocols_wlr::foreign_toplevel::v1::client::{zwlr_foreign_toplevel_manager_v1 as Manager, zwlr_foreign_toplevel_handle_v1 as Handle};

struct AppData;
struct HandleData {
    title: String,
    // do_swap: bool,
    // pa_source: String,
}

#[derive(Serialize, Deserialize)]
struct PaSource {
    index: u32,
    name: String,
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
            let user_data: Option<&HandleWrapper> = toplevel.data();

            match user_data {
                Some(handle_data) => {
                    data.write().unwrap().push(handle_data.clone());
                },
                None => panic!("Failed to get user data from toplevel handle"),
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

    // init commands

    // absolute bs of a command, will be replaced with pipewire crate later
    let loopback_id_raw = std::process::Command::new("sh")
        .args([
            "-c",
            "pactl -f json list source-outputs | jq '.[] | select(.properties.\"node.group\" | select(type==\"string\") | test(\"^loopback.*$\")) | .index'",
            // "pactl -f json list source-outputs"
        ])
        .output().expect("failed to get loopback ID").stdout;

    let loopback_id = match str::from_utf8(&loopback_id_raw) {
        Ok(v) => v,
        Err(e) => panic!("wtf happened: {}", e),
    };
    assert!(loopback_id.len() > 0 && loopback_id.find('\n').is_some_and(|i| {i == loopback_id.len() - 1}));


    let pa_sources_raw = std::process::Command::new("sh")
        .args([
            "-c",
            "pactl -f json list sources",
        ])
        .output().expect("failed to get sources").stdout;
    
    let pa_sources_s = match str::from_utf8(&pa_sources_raw) {
        Ok(v) => v,
        Err(e) => panic!("Unable to convert command output to string: {}", e),
    };

    let pa_sources_json: Vec<PaSource> = serde_json::from_str(pa_sources_s).unwrap();
    pa_sources_json.iter().for_each(|pa_source| {
        println!("[{}] {}", pa_source.index, pa_source.name)
    });

    handles.read().unwrap().iter().for_each(|handle| {
        println!("{}", handle.read().unwrap().title)
    });


    // start thread for wayland events
    std::thread::spawn(move || {
        loop {
            let _ = event_queue.blocking_dispatch(&mut AppData);
        }
    });

    // todo: add logic for exit
    let stdin = std::io::stdin();
    let mut buf = String::new();
    loop {
        let _ = stdin.read_line(&mut buf);
    }
}
