use std::collections::HashMap;

pub trait UiStore:
    Clone
    + Default
    + Send
    + Sync
    + 'static
    + std::fmt::Debug
    + serde::Serialize
    + for<'de> serde::Deserialize<'de>
{
    fn ui_get_last_action_message_id(&self) -> Option<i32>;
    fn ui_set_last_action_message_id(&mut self, id: Option<i32>);

    fn ui_get_input_prompt_message_id(&self) -> Option<i32>;
    fn ui_set_input_prompt_message_id(&mut self, id: Option<i32>);

    fn ui_get_reply_to_last_once(&self) -> bool;
    fn ui_set_reply_to_last_once(&mut self, v: bool);

    fn ui_set_scene_for_message(&mut self, message_id: i32, scene_json: String);
    fn ui_get_scene_for_message(&self, message_id: i32) -> Option<String>;

    fn ui_get_current_scene_json(&self) -> String;

    fn ui_get_active_scene_id(&self) -> Option<String>;
    fn ui_set_active_scene_id(&mut self, id: Option<String>);
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SimpleSession {
    active_scene_id: Option<String>,
    last_message_id: Option<i32>,
    input_prompt_message_id: Option<i32>,
    reply_to_last_once: bool,
    message_scenes: HashMap<i32, String>,
}

impl UiStore for SimpleSession {
    fn ui_get_last_action_message_id(&self) -> Option<i32> {
        self.last_message_id
    }

    fn ui_set_last_action_message_id(&mut self, id: Option<i32>) {
        self.last_message_id = id;
    }

    fn ui_get_input_prompt_message_id(&self) -> Option<i32> {
        self.input_prompt_message_id
    }

    fn ui_set_input_prompt_message_id(&mut self, id: Option<i32>) {
        self.input_prompt_message_id = id;
    }

    fn ui_get_reply_to_last_once(&self) -> bool {
        self.reply_to_last_once
    }

    fn ui_set_reply_to_last_once(&mut self, v: bool) {
        self.reply_to_last_once = v;
    }

    fn ui_set_scene_for_message(&mut self, message_id: i32, scene_json: String) {
        // Keep structure simple; last wins
        self.message_scenes.insert(message_id, scene_json);
    }

    fn ui_get_scene_for_message(&self, message_id: i32) -> Option<String> {
        self.message_scenes.get(&message_id).cloned()
    }

    fn ui_get_current_scene_json(&self) -> String {
        match self.active_scene_id.as_deref() {
            Some(id) => format!("\"{id}\""),
            None => "null".to_string(),
        }
    }

    fn ui_get_active_scene_id(&self) -> Option<String> {
        self.active_scene_id.clone()
    }

    fn ui_set_active_scene_id(&mut self, id: Option<String>) {
        self.active_scene_id = id;
    }
}
