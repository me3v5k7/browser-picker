// Helpers for talking to the host system, which matters when running inside a flatpak sandbox:
// there the host's XDG dirs are hidden/remapped and host binaries can't be run directly.
use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*;
use std::collections::HashMap;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

pub fn is_flatpak() -> bool {
    static IS_FLATPAK: OnceLock<bool> = OnceLock::new();
    *IS_FLATPAK.get_or_init(|| Path::new("/.flatpak-info").exists())
}

// XDG data dirs of the host in order of precedence (XDG_DATA_HOME first, then XDG_DATA_DIRS)
pub fn data_dirs() -> &'static [PathBuf] {
    static DATA_DIRS: OnceLock<Vec<PathBuf>> = OnceLock::new();
    DATA_DIRS.get_or_init(|| {
        let (home, data_home, data_dirs) = if is_flatpak() {
            // The sandbox has its own XDG_* values, so ask the host for the real ones
            let script = r#"printf '%s\0%s\0%s' "$HOME" "$XDG_DATA_HOME" "$XDG_DATA_DIRS""#;
            let output = capture(script, &[]).unwrap_or_default();
            let output = String::from_utf8_lossy(&output);
            let mut values = output.split('\0').map(String::from);
            (
                values.next().unwrap_or_default(),
                values.next().unwrap_or_default(),
                values.next().unwrap_or_default(),
            )
        } else {
            (
                glib::home_dir().to_string_lossy().to_string(),
                std::env::var("XDG_DATA_HOME").unwrap_or_default(),
                std::env::var("XDG_DATA_DIRS").unwrap_or_default(),
            )
        };

        build_data_dirs(&home, &data_home, &data_dirs)
    })
}

fn build_data_dirs(home: &str, data_home: &str, data_dirs: &str) -> Vec<PathBuf> {
    // Defaults as defined by the XDG Base Directory spec
    let data_home = if data_home.is_empty() && !home.is_empty() {
        format!("{}/.local/share", home)
    } else {
        data_home.to_string()
    };
    let data_dirs = if data_dirs.is_empty() {
        "/usr/local/share:/usr/share"
    } else {
        data_dirs
    };

    let mut dirs: Vec<PathBuf> = Vec::new();
    for dir in std::iter::once(data_home.as_str()).chain(data_dirs.split(':')) {
        let dir = PathBuf::from(dir.trim_end_matches('/'));
        // The spec says relative paths must be ignored
        if dir.is_absolute() && !dirs.contains(&dir) {
            dirs.push(dir);
        }
    }

    if dirs.is_empty() {
        println!("Could not determine any XDG data dirs");
    }
    dirs
}

// Runs a shell script on the host (flatpak only) and returns its stdout
pub fn capture(script: &str, args: &[&str]) -> Option<Vec<u8>> {
    let output = Command::new("flatpak-spawn")
        .args(["--host", "sh", "-c", script, "sh"])
        .args(args)
        .output();

    match output {
        Ok(output) if output.status.success() => Some(output.stdout),
        Ok(output) => {
            if !output.stderr.is_empty() {
                println!("Host command failed: {}", String::from_utf8_lossy(&output.stderr));
            }
            None
        }
        Err(err) => {
            println!("Error running flatpak-spawn: {}", err);
            None
        }
    }
}

// Starts a process on the host (flatpak only). This calls the same D-Bus method that
// flatpak-spawn uses, but returns as soon as the process is started, so the picker can quit
// right away without the sandbox teardown killing the launch.
pub fn spawn(argv: &[String], cwd: &Path) -> Result<(), glib::Error> {
    // The API expects nul-terminated bytestrings
    let bytestring = |bytes: &[u8]| {
        let mut bytes = bytes.to_vec();
        bytes.push(0);
        bytes
    };

    let cwd = bytestring(cwd.as_os_str().as_bytes());
    let argv: Vec<Vec<u8>> = argv.iter().map(|arg| bytestring(arg.as_bytes())).collect();
    let fds: HashMap<u32, glib::variant::Handle> = HashMap::new();
    let envs: HashMap<String, String> = HashMap::new();
    let flags: u32 = 0;

    let bus = gio::bus_get_sync(gio::BusType::Session, gio::Cancellable::NONE)?;
    bus.call_sync(
        Some("org.freedesktop.Flatpak"),
        "/org/freedesktop/Flatpak/Development",
        "org.freedesktop.Flatpak.Development",
        "HostCommand",
        Some(&(cwd, argv, fds, envs, flags).to_variant()),
        None,
        gio::DBusCallFlags::NONE,
        -1,
        gio::Cancellable::NONE,
    )?;

    Ok(())
}
