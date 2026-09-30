use gtk4::glib;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// Helper struct to store the details of each action
#[derive(Debug, Clone, Default)]
pub struct DesktopAction {
    pub name: Option<String>,
    pub exec: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct DesktopApp {
    // Core Identifiers
    pub path: PathBuf,            // File path of the .desktop file
    pub name: String,             // Name=
    pub version: Option<String>,  // Version= (Spec version, e.g., "1.5")
    pub entry_type: Option<String>,// Type= (e.g., "Application", "Link")
    pub comment: Option<String>,  // Comment= (Description/Tooltip)
    pub icon: Option<String>,     // Icon= (Icon name or absolute path)
    pub exec: Option<String>,     // Exec= (Executable command string)

    pub categories: Vec<String>,  // Categories=
    pub actions: HashMap<String, DesktopAction>,

    pub mime_types: Vec<String>,  // MimeType= (e.g., "text/html;x-scheme-handler/http;")
    pub keywords: Vec<String>,    // Keywords= (Search terms for application launchers)

    // Additional Useful Spec Metadata
    pub work_path: Option<String>,// Path= (Working directory to run the binary in)
    pub startup_wm_class: Option<String>, // StartupWMClass= (Used by docks to group windows)
    pub terminal: bool,           // Terminal= (true if app runs inside a CLI terminal)
    pub no_display: bool,         // NoDisplay= (true if app is hidden from menus)
    pub hidden: bool,             // Hidden= (true if file was deleted/disabled by user)
}

pub fn parse_and_filter_app(path: &Path, contents: &str, target_categories: &[&str], show_hidden_and_nodisplay: bool) -> Option<DesktopApp> {
    let mut app = DesktopApp {
        path: path.to_path_buf(),
        ..Default::default()
    };

    // Track which [Section] we are currently reading
    let mut current_section = String::new();

    for line in contents.lines() {
        let line = line.trim();

        // Skip blank lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Update the current section when we hit a [Bracketed Name]
        if line.starts_with('[') && line.ends_with(']') {
            current_section = line[1..line.len() - 1].to_string();
            continue;
        }

        if let Some((key, val)) = line.split_once('=') {
            let val = val.trim(); // Trim any accidental whitespace around the value

            if current_section == "Desktop Entry" {
                match key {
                    "Name" => app.name = val.to_string(),
                    "Version" => app.version = Some(val.to_string()),
                    "Type" => app.entry_type = Some(val.to_string()),
                    "Comment" => app.comment = Some(val.to_string()),
                    "Icon" => app.icon = Some(val.to_string()),
                    "Exec" => app.exec = Some(val.to_string()),
                    "Path" => app.work_path = Some(val.to_string()),
                    "StartupWMClass" => app.startup_wm_class = Some(val.to_string()),
                    "Terminal" => app.terminal = val == "true",
                    "NoDisplay" => app.no_display = val == "true",
                    "Hidden" => app.hidden = val == "true",

                    // Semicolon-delimited fields
                    "Categories" => app.categories = parse_list(val),
                    "MimeType" => app.mime_types = parse_list(val),
                    "Keywords" => app.keywords = parse_list(val),

                    _ => {} // Ignore unneeded or unknown keys
                }
            }
            // If the section is "Desktop Action <ActionID>" (e.g., "Desktop Action new-private-window")
            else if let Some(action_id) = current_section.strip_prefix("Desktop Action ") {
                // Get the existing action struct, or insert a new empty one if this is the first key we found for it
                let action = app.actions
                    .entry(action_id.to_string())
                    .or_insert_with(DesktopAction::default);

                match key {
                    "Name" => action.name = Some(val.to_string()),
                    "Exec" => action.exec = Some(val.to_string()),
                    _ => {} // Ignore other action keys like Icon if you don't need them
                }
            }
        }
    }

    // Check category match any
    // let matches_category = app
    //     .categories
    //     .iter()
    //     .any(|cat| target_categories.contains(&cat.as_str()));

    // Check category match all
    let matches_category = target_categories
        .iter()
        .all(|target| app.categories.iter().any(|cat| cat.as_str() == *target));

    // Ensure it's valid, matches categories, and isn't marked as hidden/no-display
    if !app.name.is_empty() && matches_category && (!app.hidden && !app.no_display || show_hidden_and_nodisplay) {
        Some(app)
    } else {
        None
    }
}

// Helper to split semicolon-separated lists
fn parse_list(raw_value: &str) -> Vec<String> {
    raw_value
        .split(';')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

// Turns an Exec= value into an argument list, replacing the field codes as described in
// https://specifications.freedesktop.org/desktop-entry-spec/latest/exec-variables.html
pub fn expand_exec(exec: &str, url: &str, app: &DesktopApp) -> Option<Vec<String>> {
    // Undo the general string escaping first, the rest of the quoting rules match the shell ones
    let mut unescaped = String::new();
    let mut chars = exec.chars();
    while let Some(c) = chars.next() {
        match (c, chars.clone().next()) {
            ('\\', Some('\\')) => { unescaped.push('\\'); chars.next(); }
            ('\\', Some('s')) => { unescaped.push(' '); chars.next(); }
            _ => unescaped.push(c),
        }
    }
    let exec = unescaped;
    let parsed = glib::shell_parse_argv(&exec)
        .inspect_err(|err| println!("Error parsing Exec={}: {}", exec, err))
        .ok()?;

    let mut argv = Vec::new();
    let mut has_url_code = false;

    for arg in parsed {
        let arg = arg.to_string_lossy();

        // Codes that take up a whole argument and may expand to zero or several arguments
        match arg.as_ref() {
            "%u" | "%U" | "%f" | "%F" => {
                has_url_code = true;
                if !url.is_empty() {
                    argv.push(url.to_string());
                }
                continue;
            }
            "%i" => {
                if let Some(icon) = &app.icon {
                    argv.push("--icon".to_string());
                    argv.push(icon.clone());
                }
                continue;
            }
            _ => {}
        }

        let mut expanded = String::new();
        let mut chars = arg.chars();
        while let Some(c) = chars.next() {
            if c != '%' {
                expanded.push(c);
                continue;
            }
            match chars.next() {
                Some('%') => expanded.push('%'),
                Some('u' | 'U' | 'f' | 'F') => {
                    has_url_code = true;
                    expanded.push_str(url);
                }
                Some('c') => expanded.push_str(&app.name),
                Some('k') => expanded.push_str(&app.path.to_string_lossy()),
                _ => {} // Deprecated or unknown codes are removed
            }
        }
        argv.push(expanded);
    }

    // Some actions (e.g. "firefox --private-window") don't take a url, pass it anyway
    if !has_url_code && !url.is_empty() {
        argv.push(url.to_string());
    }

    if argv.is_empty() { None } else { Some(argv) }
}
