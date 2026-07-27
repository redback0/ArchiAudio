
use crate::Action;

pub struct CommandAction {
    comm: std::process::Command,
}

impl CommandAction {
    pub fn new(comm: std::process::Command) -> CommandAction {
        CommandAction{comm: comm}
    }
}

impl Action for CommandAction {
    fn trigger(&mut self) -> Result<(), String> {
        match self.comm.spawn() {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to run command: {}", e)),
        }
    }
}
