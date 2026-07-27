use serde::{Deserialize, Serialize};

use crate::Action;

pub struct PactlAction {
    comm: std::process::Command,
}

pub struct PactlActionGenerator {
    loopback_id: String,
    pa_sources: Vec<PaSource>,
}

#[derive(Serialize, Deserialize)]
struct PaSource {
    index: u32,
    name: String,
}

impl PactlActionGenerator {
    pub fn new() -> Self {
        // absolute bs of a command, will be replaced with pipewire crate later
        let loopback_id_raw = std::process::Command::new("sh")
            .args([
                "-c",
                "pactl -f json list source-outputs | jq '.[] | select(.properties.\"node.group\" | select(type==\"string\") | test(\"^loopback.*$\")) | .index'",
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

        Self{loopback_id: loopback_id.to_string(), pa_sources: pa_sources_json}
    }


    pub fn build_action(&self) -> Result<PactlAction, String> {
        let stdin = std::io::stdin();
        let mut buf = std::string::String::new();


        self.pa_sources.iter().enumerate().for_each(|(i, pa_source)| {
            println!("[{}] {}", i, pa_source.name)
        });

        println!("\nWhich source would you to swap to?");
        buf.clear();
        if stdin.read_line(&mut buf).unwrap_or(0) <= 1 {
            return Err("invalid index".to_string());
        };

        let pa_source_index = buf.trim().parse::<usize>().unwrap_or(usize::MAX);

        if pa_source_index >= self.pa_sources.len() {
            return Err("invalid index".to_string());
        }

        let mut comm = std::process::Command::new("sh");
        comm.args([
            "-c",
            format!("pactl move-source-output {} {}", self.loopback_id, self.pa_sources[pa_source_index].index).as_str(),
        ]);

        Ok(PactlAction { comm: comm })
    }
}

impl Action for PactlAction {
    fn trigger(&mut self) -> Result<(), String> {
        match self.comm.spawn() {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to run command: {}", e)),
        }
    }
}
