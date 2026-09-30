use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use crate::host;

pub struct DesktopFile {
    pub id: String,         // Desktop file ID, e.g. "org.mozilla.firefox.desktop"
    pub path: PathBuf,      // Path of the file on the host
    pub contents: String,
}

// Finds all .desktop files in <data dir><suffix> for every XDG data dir. If the same desktop file ID
// exists in several dirs, only the one from the dir with the highest precedence is kept (as per spec).
pub fn get_all_desktop_files(dirs: &[PathBuf], suffix: &str) -> Vec<DesktopFile> {
    let files = if host::is_flatpak() {
        read_host_desktop_files(dirs, suffix)
    } else {
        read_local_desktop_files(dirs, suffix)
    };

    let mut seen_ids = HashSet::new();
    files
        .into_iter()
        .filter(|file| seen_ids.insert(file.id.clone()))
        .collect()
}

fn read_local_desktop_files(dirs: &[PathBuf], suffix: &str) -> Vec<DesktopFile> {
    let mut desktop_files = Vec::new();

    for dir in dirs {
        let base_path = PathBuf::from(dir.to_string_lossy().to_string() + suffix);
        let mut paths = Vec::new();
        visit_rec_dirs(&base_path, &mut paths);

        for path in paths {
            let Ok(bytes) = fs::read(&path) else {
                println!("Error reading file {}", path.to_string_lossy());
                continue;
            };

            desktop_files.push(DesktopFile {
                id: desktop_file_id(&base_path, &path),
                contents: String::from_utf8_lossy(&bytes).to_string(),
                path,
            });
        }
    }

    return desktop_files;
}

// Inside a flatpak most host dirs are either hidden or mounted elsewhere, so the files are listed
// and read on the host in a single call
fn read_host_desktop_files(dirs: &[PathBuf], suffix: &str) -> Vec<DesktopFile> {
    // Prints "<base dir>\0<file path>\0<file contents>\0" for every desktop file
    let script = r#"
        suffix=$1; shift
        for dir in "$@"; do
            [ -d "$dir$suffix" ] || continue
            find -L "$dir$suffix" -type f -name '*.desktop' -exec sh -c '
                base=$1; shift
                for file in "$@"; do
                    printf "%s\0%s\0" "$base" "$file"
                    cat "$file"
                    printf "\0"
                done' sh "$dir$suffix" {} +
        done"#;

    let dir_strings: Vec<String> = dirs.iter().map(|d| d.to_string_lossy().to_string()).collect();
    let mut args = vec![suffix];
    args.extend(dir_strings.iter().map(String::as_str));

    let Some(output) = host::capture(script, &args) else {
        println!("Error reading desktop files from the host");
        return Vec::new();
    };

    let output = String::from_utf8_lossy(&output);
    let mut fields = output.split('\0');
    let mut desktop_files = Vec::new();

    while let (Some(base), Some(path), Some(contents)) = (fields.next(), fields.next(), fields.next()) {
        let path = PathBuf::from(path);
        desktop_files.push(DesktopFile {
            id: desktop_file_id(Path::new(base), &path),
            path,
            contents: contents.to_string(),
        });
    }

    return desktop_files;
}

// The desktop file ID is the path relative to the applications dir with "/" replaced by "-"
fn desktop_file_id(base_path: &Path, path: &Path) -> String {
    let relative = path.strip_prefix(base_path).unwrap_or(path);
    relative.to_string_lossy().replace('/', "-")
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
