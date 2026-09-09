use gtk4::gio::ApplicationFlags;
use gtk4::prelude::*;
use gtk4::Application;
use std::env;

mod browser_manager;
mod settings_manager;
mod ui_manager;

use settings_manager::BrowserPickerSettings;
use ui_manager::build_ui;

fn main() {
    let args: Vec<String> = env::args().collect();
    let url = args.get(1).cloned().unwrap_or_else(|| "".to_string());

    let settings_path = BrowserPickerSettings::get_default_path();
    let mut settings = BrowserPickerSettings::load_from_file(&settings_path);

    let found_browsers: Vec<_> = browser_manager::get_browsers();
    settings.sync_system_apps(found_browsers);
    let saving_result = settings.save_to_file(&settings_path);
    if let Err(err) = saving_result {
        println!("Error saving to path: {:?} with error: {:?}", settings_path, err);
    }

    let app = Application::builder()
        .application_id("com.me.browser-picker")
        .flags(ApplicationFlags::NON_UNIQUE)
        .build();

    let url_clone = url.clone();
    let path_clone = settings_path.clone();

    app.connect_activate(move |app| {
        build_ui(app, &url_clone, settings.clone(), path_clone.clone());
    });

    app.run_with_args(&Vec::<String>::new());
}
