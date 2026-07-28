use Box;
use wayland_client::{Connection, Dispatch, Proxy};
use wayland_protocols_wlr::foreign_toplevel::v1::client::{
    zwlr_foreign_toplevel_manager_v1 as top_level_manager,
    zwlr_foreign_toplevel_handle_v1 as top_level_handle
};

mod pactl_action;
use crate::pactl_action::PactlActionGenerator;

pub trait Action: Send + Sync {
    fn trigger(&mut self) -> Result<(), String>;
}

pub trait ActionGenerator<T>: where T: Action + ?Sized {
    fn build_action(&self) -> Result<Box<T>, String>;
    fn get_action_name(&self) -> String;
}

struct AppData;
struct TLHandleActions {
    title: String,
    curr_active: bool,
    active_actions: Vec<Box<dyn Action>>,
    deactive_actions: Vec<Box<dyn Action>>,
}

type HandleWrapper = std::sync::Arc<std::sync::RwLock<TLHandleActions>>;
type HandleVec = std::sync::Arc<std::sync::RwLock<Vec<HandleWrapper>>>;

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
        if let top_level_handle::Event::Title { title } = event {
            let mut handle_data = handle_data_lock.write().unwrap();
            handle_data.title = title;
        }
        else if let top_level_handle::Event::State { state } = event {
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
        }
    }
}

fn main() {
    let handles_lock = HandleVec::new(std::sync::RwLock::new(vec![]));

    let conn: Connection = Connection::connect_to_env().unwrap();
    let (globals, mut event_queue) = wayland_client::globals::registry_queue_init::<AppData>(&conn).unwrap();

    let _toplevel_manager: top_level_manager::ZwlrForeignToplevelManagerV1 = globals.bind(&event_queue.handle(), 3..=3, handles_lock.clone()).unwrap();

    event_queue.roundtrip(&mut AppData).unwrap();

    // ask the user what toplevels to associate with what pa sources
    let stdin = std::io::stdin();
    let mut buf = String::new();

    // create scope to avoid calling .read().unwrap() constantly
    {
        let handles = handles_lock.read().unwrap();
        let mut generators: Vec<Box<dyn ActionGenerator<dyn Action>>> = vec!();
        generators.push(Box::new(PactlActionGenerator::new()));

        loop {
            handles.iter().enumerate().for_each(|(i, handle)| {
                println!("[{}] {}", i, handle.read().unwrap().title)
            });
            println!("\nWhich window would you like to have swap? (leave empty to skip)");
            buf.clear();
            if stdin.read_line(&mut buf).unwrap_or(0) <= 1 { break };

            let handle_index = buf.trim().parse::<usize>().unwrap_or(usize::MAX);

            if handle_index >= handles.len() {
                println!("invalid index");
                continue;
            }

            println!("\n");

            generators.iter().enumerate().for_each(|(i, generator)| {
                println!("[{}] {}", i, generator.get_action_name())
            });

            buf.clear();
            if stdin.read_line(&mut buf).unwrap_or(0) <= 1 {
                println!("invalid index");
            };

            let generator_index = buf.trim().parse::<usize>().unwrap_or(usize::MAX);

            if generator_index >= generators.len() {
                println!("invalid index");
                continue;
            }

            let generator = generators[generator_index].as_ref();

            let comm = generator.build_action();

            match comm {
                Ok(v) => handles[handle_index].write().unwrap().active_actions.push(v),
                Err(e) => println!("Failed to set action: {}", e),
            }
        }
    }

    println!("setup complete");

    loop {
        event_queue.blocking_dispatch(&mut AppData).unwrap();
    }
}
