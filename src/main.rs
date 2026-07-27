use serde::{Deserialize, Serialize};
use wayland_client::{Connection, Dispatch, Proxy, event_created_child, globals};
use wayland_protocols_wlr::foreign_toplevel::v1::client::{zwlr_foreign_toplevel_manager_v1 as top_level_manager, zwlr_foreign_toplevel_handle_v1 as top_level_handle};

struct AppData;
struct HandleActions {
    title: String,
    comm: Option<std::process::Command>,
}

#[derive(Serialize, Deserialize)]
struct PaSource {
    index: u32,
    name: String,
}

type HandleWrapper = std::sync::Arc<std::sync::RwLock<HandleActions>>;
type HandleVec = std::sync::Arc<std::sync::RwLock<Vec<HandleWrapper>>>;

impl Dispatch<wayland_client::protocol::wl_registry::WlRegistry, wayland_client::globals::GlobalListContents> for AppData {
    fn event(
        _: &mut Self,
        _: &wayland_client::protocol::wl_registry::WlRegistry,
        _event: wayland_client::protocol::wl_registry::Event,
        _data: &globals::GlobalListContents,
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

    event_created_child!(AppData, top_level_manager::ZwlrForeignToplevelManagerV1, [
        top_level_manager::EVT_TOPLEVEL_OPCODE => (top_level_handle::ZwlrForeignToplevelHandleV1,
            std::sync::Arc::new(HandleActions {
                title: String::new(),
                comm: None,
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
        if let top_level_handle::Event::Title { title } = event {
            let mut handle_data = handle_data_lock.write().unwrap();
            handle_data.title = title;
        }
        else if let top_level_handle::Event::State { state } = event {
            if state.contains(&2) {
                let mut handle_data = handle_data_lock.write().unwrap();
                println!("Active: {}", handle_data.title);
                // handle audio swap here
                if let Some(comm) = handle_data.comm.as_mut() {
                    println!("{} {:?}", comm.get_program().to_str().unwrap(), comm.get_args());
                    let _ = comm.spawn();
                }
            }
        }
    }
}

fn main() {
    let handles_lock = HandleVec::new(std::sync::RwLock::new(vec![]));

    let conn: Connection = Connection::connect_to_env().unwrap();
    let (globals, mut event_queue) = globals::registry_queue_init::<AppData>(&conn).unwrap();

    let _toplevel_manager: top_level_manager::ZwlrForeignToplevelManagerV1 = globals.bind(&event_queue.handle(), 3..=3, handles_lock.clone()).unwrap();

    event_queue.roundtrip(&mut AppData).unwrap();

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
        Ok(v) => v.trim(),
        Err(e) => panic!("wtf happened: {}", e),
    };
    assert!(loopback_id.len() > 0);


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

    // all info obtained


    

    // ask the user what toplevels to associate with what pa sources
    let stdin = std::io::stdin();
    let mut buf = String::new();

    // create scope to avoid calling .read().unwrap() constantly
    {
        let handles = handles_lock.read().unwrap();

        loop {
            handles.iter().enumerate().for_each(|(i, handle)| {
                println!("[{}] {}", i, handle.read().unwrap().title)
            });
            println!("\nWhich window would you like to have swap? (leave empty to skip)");
            buf.clear();
            if stdin.read_line(&mut buf).unwrap_or(0) <= 1 { break };

            let handle_index = buf.trim().parse::<usize>().unwrap_or(usize::MAX);

            println!("{}", handle_index);

            if handle_index >= handles.len() {
                println!("invalid index");
                continue;
            }

            println!("\n");

            pa_sources_json.iter().enumerate().for_each(|(i, pa_source)| {
                println!("[{}] {}", i, pa_source.name)
            });

            println!("\nWhich source would you like to have \"{}\" swap to?", handles[handle_index].read().unwrap().title);
            buf.clear();
            if stdin.read_line(&mut buf).unwrap_or(0) <= 1 {
                println!("invalid index");
                continue;
            };

            let pa_source_index = buf.trim().parse::<usize>().unwrap_or(usize::MAX);

            if pa_source_index >= pa_sources_json.len() {
                println!("invalid index");
                continue;
            }

            let mut comm = std::process::Command::new("sh");
            comm.args([
                "-c",
                format!("pactl move-source-output {} {}", loopback_id, pa_sources_json[pa_source_index].index.to_string().as_str()).as_str(),
            ]);

            handles[handle_index].write().unwrap().comm = Some(comm);
        }

        println!("\n");
    }

    println!("setup complete");

    // start thread for wayland events
    // let (tx, rx) = std::sync::mpsc::channel();

    std::thread::spawn(move || {
        loop {
            event_queue.blocking_dispatch(&mut AppData).unwrap();
        }
    });


    loop {
        if stdin.read_line(&mut buf).unwrap_or(0) > 0 && buf.trim() == "exit" { break };
    }

    // println!("exiting...");
    // let _ = tx.send(());
}
