use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

pub fn get_all_desktop_files(dirs: &HashSet<String>, suffix: &str) -> Vec<PathBuf> {
    let mut desktop_files = Vec::new();

    for dir in dirs {
        let combined_path_string = String::from(dir) + suffix;
        let path = Path::new(&combined_path_string);
        visit_rec_dirs(path, &mut desktop_files);
    }

    return desktop_files;
}

fn visit_rec_dirs(dir: &Path, desktop_files: &mut Vec<PathBuf>) {
    if !dir.is_dir() {
        return;
    }

    let files_result = fs::read_dir(dir);
    if files_result.is_err() {
        println!("Error reading dir {}", dir.to_string_lossy());
        return;
    }
    let files = files_result.unwrap();

    for file_result in files {

        if let Ok(file) = file_result {
            let path = file.path();

            if path.is_dir() {
                visit_rec_dirs(&path, desktop_files);
            } else if path.extension() == Some(OsStr::new("desktop")) {
                desktop_files.push(path);
            }
        }
    }
}


pub fn get_desktop_dirs() -> HashSet<String> {
    let xdg_data_dirs_result = std::env::var("XDG_DATA_DIRS");
    let xdg_data_home_result = std::env::var("XDG_DATA_HOME");

    if xdg_data_dirs_result.is_err() {
        println!("Environment variable XDG_DATA_DIRS not defined");
    }
    let xdg_data_dirs_string = xdg_data_dirs_result.unwrap_or(String::from(""));

    if xdg_data_home_result.is_err() {
        println!("Environment variable XDG_DATA_HOME not defined");
    }
    let xdg_data_home_string = xdg_data_home_result.unwrap_or(String::from(""));

    let xdg_data_dirs = xdg_data_dirs_string.split(":");
    let xdg_data_home = xdg_data_home_string.split(":");

    let desktop_dirs: HashSet<String> = xdg_data_dirs
        .chain(xdg_data_home)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();

    return desktop_dirs;
}
