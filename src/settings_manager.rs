use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::browser_manager::desktop_parser::DesktopApp;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BrowserPickerSettings {
    pub browser_order: Vec<String>,
    pub browsers: HashMap<String, BrowserSettings>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserSettings {
    #[serde(default)]
    pub saved_name: String,
    #[serde(skip)]
    pub app_data: DesktopApp,
    #[serde(skip)]
    pub is_installed: bool,
    pub is_visible: bool,
    pub actions: Vec<ActionSetting>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionSetting {
    pub id: String,
    pub is_visible: bool,
}

impl BrowserSettings {
    pub fn move_action(&mut self, from_index: usize, to_index: usize) {
        if from_index < self.actions.len() && to_index < self.actions.len() {
            let item = self.actions.remove(from_index);
            self.actions.insert(to_index, item);
        }
    }
}

impl BrowserPickerSettings {
    pub fn get_default_path() -> PathBuf {
        if let Some(mut path) = gtk4::glib::user_config_dir().to_str().map(PathBuf::from) {
            path.push("browser-picker");
            let _ = fs::create_dir_all(&path);
            path.push("settings.json");
            path
        } else {
            PathBuf::from("my_browser_settings.json")
        }
    }

    pub fn load_from_file(file_path: &Path) -> Self {
        if let Ok(json_data) = fs::read_to_string(file_path) {
            let mut settings: Self = serde_json::from_str(&json_data).unwrap_or_default();

            for key in settings.browsers.keys() {
                if !settings.browser_order.contains(key) {
                    settings.browser_order.push(key.clone());
                }
            }
            settings
        } else {
            Self::default()
        }
    }

    pub fn save_to_file(&self, file_path: &Path) -> Result<(), std::io::Error> {
        if let Some(parent) = file_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let json_data = serde_json::to_string_pretty(self)?;
        fs::write(file_path, json_data)
    }

    pub fn sync_system_apps(&mut self, found_apps: Vec<DesktopApp>) {
        // Reset installed flag for all configured browsers
        for setting in self.browsers.values_mut() {
            setting.is_installed = false;
        }

        for app in found_apps {
            let path_key = app.path.to_string_lossy().to_string();

            if let Some(existing_setting) = self.browsers.get_mut(&path_key) {
                existing_setting.app_data = app.clone();
                existing_setting.saved_name = app.name.clone();
                existing_setting.is_installed = true;

                let mut current_os_actions = HashSet::new();

                for (action_id, action_details) in &existing_setting.app_data.actions {
                    current_os_actions.insert(action_id.clone());

                    if !existing_setting.actions.iter().any(|a| a.id == *action_id) {
                        let is_private = is_private_action(action_id, action_details.name.as_deref());

                        existing_setting.actions.push(ActionSetting {
                            id: action_id.clone(),
                            is_visible: is_private,
                        });
                    }
                }

                existing_setting.actions.retain(|a| current_os_actions.contains(&a.id));
            } else {
                let mut actions = Vec::new();

                for (action_id, action_details) in &app.actions {
                    let is_private = is_private_action(action_id, action_details.name.as_deref());

                    actions.push(ActionSetting {
                        id: action_id.clone(),
                        is_visible: is_private,
                    });
                }

                let app_name = app.name.clone();
                self.browsers.insert(
                    path_key.clone(),
                    BrowserSettings {
                        saved_name: app_name,
                        app_data: app,
                        is_installed: true,
                        is_visible: true,
                        actions,
                    },
                );

                self.browser_order.push(path_key);
            }
        }
    }

    pub fn move_browser(&mut self, from_index: usize, to_index: usize) {
        if from_index < self.browser_order.len() && to_index < self.browser_order.len() {
            let item = self.browser_order.remove(from_index);
            self.browser_order.insert(to_index, item);
        }
    }

    pub fn remove_browser(&mut self, path_key: &str) {
        self.browsers.remove(path_key);
        self.browser_order.retain(|k| k != path_key);
    }
}

fn is_private_action(action_id: &str, action_name: Option<&str>) -> bool {
    let id_lower = action_id.to_lowercase();
    let name_lower = action_name.unwrap_or("").to_lowercase();

    id_lower.contains("private")
        || id_lower.contains("incognito")
        || name_lower.contains("private")
        || name_lower.contains("incognito")
}
