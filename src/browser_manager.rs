mod desktop_finder;
pub mod desktop_parser;

pub fn get_browsers() -> Vec<desktop_parser::DesktopApp>{
    let desktop_dirs = desktop_finder::get_desktop_dirs();

    let files = desktop_finder::get_all_desktop_files(&desktop_dirs, "/applications");

    let targets = ["WebBrowser"];

    let browser_apps: Vec<desktop_parser::DesktopApp> = files
            .iter()
            .filter_map(|path| desktop_parser::parse_and_filter_app(path, &targets, false))
            .collect();

    return browser_apps;
}
