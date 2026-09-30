mod desktop_finder;
pub mod desktop_parser;

use crate::host;

// Our own desktop file is also in the WebBrowser category, don't list it
const OWN_DESKTOP_ID: &str = "com.me.browser-picker.desktop";

pub fn get_browsers() -> Vec<desktop_parser::DesktopApp>{
    let desktop_dirs = host::data_dirs();

    let files = desktop_finder::get_all_desktop_files(desktop_dirs, "/applications");

    let targets = ["WebBrowser"];

    let browser_apps: Vec<desktop_parser::DesktopApp> = files
            .iter()
            .filter(|file| file.id != OWN_DESKTOP_ID)
            .filter_map(|file| desktop_parser::parse_and_filter_app(&file.path, &file.contents, &targets, false))
            .collect();

    return browser_apps;
}
