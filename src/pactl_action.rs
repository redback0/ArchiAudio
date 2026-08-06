use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};

use crate::{Action, ActionDataID, ActionGenerator, GeneratorIndex, IcedMessage};

pub struct PactlAction {
    comm: Option<std::process::Command>,
    source: Option<ActionDataID>,
    generator: GeneratorIndex,
}

pub struct PactlActionGenerator {
    loopback_id: String,
    // pa_sources: Vec<PaSource>,
    action_datas: iced::widget::combo_box::State<ActionDataID>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PaSource {
    index: u32,
    name: String,
}

impl Action for PactlAction {
    fn trigger(&mut self) -> Result<(), String> {
        let comm = match self.comm.as_mut() {
            Some(c) => c,
            _ => return Ok(()),
        };
        match comm.spawn() {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to run command: {}", e)),
        }
    }
    
    fn get_generator(&self) -> GeneratorIndex {
        self.generator.clone()
    }
    
    fn view<'a>(&self, self_lock: Arc<RwLock<dyn Action>>, generators: &'a Vec<Arc<dyn ActionGenerator<dyn Action>>>) -> iced::Element<'a, IcedMessage> {
        let move_self_lock = self_lock.clone();
        let action_data = self.source.clone();
        let gener: &PactlActionGenerator = generators[self.generator.index].as_any().downcast_ref().unwrap();
        iced::widget::column![
            iced::widget::combo_box(
                &gener.action_datas,
                "asdf",
                action_data.as_ref(),
                move |change| IcedMessage::UpdateAction(
                    move_self_lock.clone(),
                    change,
                )
            )
        ].width(200).into()
    }
    
    fn update(&mut self, new_data: ActionDataID, generators: &Vec<Arc<dyn ActionGenerator<dyn Action>>>) {
        let gener: &PactlActionGenerator = generators[self.generator.index].as_any().downcast_ref().unwrap();

        let mut comm = std::process::Command::new("sh");
        comm.args([
            "-c",
            format!("pactl move-source-output {} {}", gener.loopback_id, new_data.id).as_str(),
        ]);
        self.comm = Some(comm);
        self.source = Some(new_data);
    }
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
            Err(e) => panic!("Command output failed to interpret as str: {}", e),
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

        let pa_sources: Vec<PaSource> = serde_json::from_str(pa_sources_s).unwrap();

        // let pa_sources_json: Vec<PaSource> = serde_json::from_str(pa_sources_s).unwrap();
        let action_datas = iced::widget::combo_box::State::<ActionDataID>::new(pa_sources
            .iter()
            .map(|s| ActionDataID {
                _option_idx: 0,
                id: s.index.into(),
                description: s.name.clone(),
            }).collect());

        Self{loopback_id: loopback_id.to_string(), action_datas}
    }
}

impl std::fmt::Display for PactlActionGenerator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Pactl Action")
    }
}

impl std::fmt::Display for PaSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name)
    }
}

impl ActionGenerator<dyn Action> for PactlActionGenerator {
    fn build_action(self: Arc<PactlActionGenerator>, gener_idx: GeneratorIndex) -> Result<Arc<RwLock<dyn Action>>, String> {
        Ok(Arc::new(RwLock::new(PactlAction { comm: None, source: None, generator: gener_idx })))
    }

    fn get_action_name(&self) -> String {
        "pactl move source".to_string()
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
