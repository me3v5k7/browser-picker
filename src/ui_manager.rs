use gtk4::gdk;
use gtk4::gio::{AppInfo, AppInfoCreateFlags};
use gtk4::glib;
use gtk4::pango::{EllipsizeMode, WrapMode};
use gtk4::prelude::*;
use gtk4::{
    Align, Application, ApplicationWindow, Box as GtkBox, Button, GestureClick, Image, Justification,
    Label, Orientation, ToggleButton,
};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use crate::browser_manager::desktop_parser::DesktopApp;
use crate::host;
use crate::settings_manager::BrowserPickerSettings;

pub fn build_ui(app: &Application, url: &str, settings: BrowserPickerSettings, settings_path: PathBuf) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Open with")
        .default_width(480)
        .resizable(false)
        .build();

    let app_for_activity = app.clone();
    window.connect_is_active_notify(move |win| {
        if !win.is_active() {
            app_for_activity.quit();
        }
    });

    let main_vbox = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(12)
        .margin_top(16)
        .margin_bottom(16)
        .margin_start(16)
        .margin_end(16)
        .build();

    let label_url = Label::builder()
        .label(if url.is_empty() { "No link provided" } else { url })
        .ellipsize(EllipsizeMode::End)
        .selectable(true)
        .halign(Align::Center)
        .margin_bottom(4)
        .build();

    label_url.add_css_class("title-4");
    label_url.add_css_class("bold");

    let is_expanded = Rc::new(Cell::new(false));
    let gesture = GestureClick::new();
    let label_clone = label_url.clone();
    gesture.connect_pressed(move |_, _, _, _| {
        let expanded = !is_expanded.get();
        is_expanded.set(expanded);
        if expanded {
            label_clone.set_ellipsize(EllipsizeMode::None);
            label_clone.set_wrap(true);
        } else {
            label_clone.set_ellipsize(EllipsizeMode::End);
            label_clone.set_wrap(false);
        }
    });
    label_url.add_controller(gesture);

    main_vbox.append(&label_url);

    let edit_bar_box = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .halign(Align::End)
        .build();

    let edit_toggle = ToggleButton::with_label("Edit Settings");
    edit_bar_box.append(&edit_toggle);
    main_vbox.append(&edit_bar_box);

    let list_container = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .build();
    main_vbox.append(&list_container);

    let settings_rc = Rc::new(RefCell::new(settings));
    let edit_mode_rc = Rc::new(Cell::new(false));
    let url_rc = Rc::new(url.to_string());
    let path_rc = Rc::new(settings_path);

    let list_container_clone = list_container.clone();
    let app_clone = app.clone();
    let settings_clone = settings_rc.clone();
    let edit_mode_clone = edit_mode_rc.clone();
    let url_clone = url_rc.clone();
    let path_save_clone = path_rc.clone();

    edit_toggle.connect_toggled(move |btn| {
        let active = btn.is_active();
        edit_mode_clone.set(active);

        if active {
            btn.set_label("Save settings");
            btn.add_css_class("suggested-action");
        } else {
            btn.set_label("Edit Settings");
            btn.remove_css_class("suggested-action");
            if let Err(e) = settings_clone.borrow().save_to_file(&path_save_clone) {
                eprintln!("Failed to save settings: {}", e);
            }
        }

        draw_browser_list(
            &list_container_clone,
            &app_clone,
            &url_clone,
            &settings_clone,
            &path_save_clone,
            edit_mode_clone.get(),
        );
    });

    draw_browser_list(&list_container, app, &url_rc, &settings_rc, &path_rc, false);

    window.set_child(Some(&main_vbox));
    window.present();
}

fn draw_browser_list(
    container: &GtkBox,
    app: &Application,
    url: &Rc<String>,
    settings_rc: &Rc<RefCell<BrowserPickerSettings>>,
    settings_path: &Rc<PathBuf>,
    is_edit_mode: bool,
) {
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }

    let settings = settings_rc.borrow();

    for (browser_idx, path_key) in settings.browser_order.iter().enumerate() {
        if let Some(browser_setting) = settings.browsers.get(path_key) {
            let is_installed = browser_setting.is_installed;

            if !is_edit_mode && (!browser_setting.is_visible || !is_installed) {
                continue;
            }

            let row_box = GtkBox::builder()
                .orientation(Orientation::Horizontal)
                .spacing(8)
                .hexpand(true)
                .build();

            if is_edit_mode && (!browser_setting.is_visible || !is_installed) {
                row_box.set_opacity(0.6);
            }

            // 1. Browser Order Buttons (Up / Down)
            if is_edit_mode {
                let order_box = GtkBox::builder().orientation(Orientation::Vertical).build();
                let btn_up = Button::with_label("▲");
                let btn_down = Button::with_label("▼");

                btn_up.add_css_class("flat");
                btn_down.add_css_class("flat");

                let s_clone = settings_rc.clone();
                let c_clone = container.clone();
                let app_c = app.clone();
                let u_clone = url.clone();
                let p_clone = settings_path.clone();

                btn_up.connect_clicked(move |_| {
                    if browser_idx > 0 {
                        s_clone.borrow_mut().move_browser(browser_idx, browser_idx - 1);
                        draw_browser_list(&c_clone, &app_c, &u_clone, &s_clone, &p_clone, true);
                    }
                });

                let s_clone2 = settings_rc.clone();
                let c_clone2 = container.clone();
                let app_c2 = app.clone();
                let u_clone2 = url.clone();
                let p_clone2 = settings_path.clone();
                let max_len = settings.browser_order.len();

                btn_down.connect_clicked(move |_| {
                    if browser_idx + 1 < max_len {
                        s_clone2.borrow_mut().move_browser(browser_idx, browser_idx + 1);
                        draw_browser_list(&c_clone2, &app_c2, &u_clone2, &s_clone2, &p_clone2, true);
                    }
                });

                order_box.append(&btn_up);
                order_box.append(&btn_down);
                row_box.append(&order_box);
            }

            // 2. Main Browser Button
            let display_name = if !browser_setting.saved_name.is_empty() {
                &browser_setting.saved_name
            } else if !browser_setting.app_data.name.is_empty() {
                &browser_setting.app_data.name
            } else {
                path_key.as_str()
            };

            let btn_browser = Button::builder()
                .hexpand(true)
                .height_request(48)
                .valign(Align::Center)
                .build();

            let btn_content = GtkBox::builder()
                .orientation(Orientation::Horizontal)
                .spacing(10)
                .margin_start(6)
                .margin_end(6)
                .build();

            if is_installed {
                if let Some(ref icon_name) = browser_setting.app_data.icon {
                    let image = load_icon(icon_name);
                    image.set_pixel_size(24);
                    btn_content.append(&image);
                }
            } else {
                let image = Image::from_icon_name("dialog-warning-symbolic");
                image.set_pixel_size(24);
                btn_content.append(&image);
            }

            let name_label = Label::new(Some(display_name));
            name_label.set_hexpand(true);
            name_label.set_halign(Align::Start);
            btn_content.append(&name_label);

            if !is_installed {
                let uninstalled_badge = Label::new(Some("(Uninstalled)"));
                uninstalled_badge.add_css_class("error");
                uninstalled_badge.set_halign(Align::End);
                btn_content.append(&uninstalled_badge);
            }

            btn_browser.set_child(Some(&btn_content));

            let s_clone = settings_rc.clone();
            let c_clone = container.clone();
            let app_clone = app.clone();
            let url_clone = url.clone();
            let path_clone = path_key.clone();
            let p_clone = settings_path.clone();
            let app_data = browser_setting.app_data.clone();

            btn_browser.connect_clicked(move |_| {
                if is_edit_mode {
                    if let Some(b) = s_clone.borrow_mut().browsers.get_mut(&path_clone) {
                        b.is_visible = !b.is_visible;
                    }
                    draw_browser_list(&c_clone, &app_clone, &url_clone, &s_clone, &p_clone, true);
                } else if is_installed {
                    launch_app(&app_clone, &url_clone, &app_data, None);
                }
            });

            if is_edit_mode {
                // Show which desktop file this entry comes from above the browser button
                let browser_box = GtkBox::builder()
                    .orientation(Orientation::Vertical)
                    .spacing(2)
                    .hexpand(true)
                    .valign(Align::Center)
                    .build();

                let path_label = Label::builder()
                    .label(path_key.as_str())
                    .tooltip_text(path_key.as_str())
                    .ellipsize(EllipsizeMode::Middle)
                    .halign(Align::Start)
                    .build();
                path_label.add_css_class("caption");
                path_label.add_css_class("dim-label");

                browser_box.append(&path_label);
                browser_box.append(&btn_browser);
                row_box.append(&browser_box);
            } else {
                row_box.append(&btn_browser);
            }

            // 3. Right Side Controls
            if !is_installed {
                let btn_remove = Button::builder()
                    .label("Remove")
                    .height_request(48)
                    .valign(Align::Center)
                    .build();
                btn_remove.add_css_class("destructive-action");

                let s_rem = settings_rc.clone();
                let c_rem = container.clone();
                let app_rem = app.clone();
                let u_rem = url.clone();
                let p_rem = settings_path.clone();
                let path_rem = path_key.clone();

                btn_remove.connect_clicked(move |_| {
                    s_rem.borrow_mut().remove_browser(&path_rem);
                    draw_browser_list(&c_rem, &app_rem, &u_rem, &s_rem, &p_rem, true);
                });

                row_box.append(&btn_remove);
            } else {
                let visible_actions: Vec<_> = browser_setting
                    .actions
                    .iter()
                    .enumerate()
                    .filter(|(_, action_config)| is_edit_mode || action_config.is_visible)
                    .collect();

                if !visible_actions.is_empty() {
                    let actions_box = GtkBox::builder()
                        .orientation(Orientation::Horizontal)
                        .spacing(6)
                        .halign(Align::End)
                        .valign(Align::Center)
                        .build();

                    let actions_len = browser_setting.actions.len();

                    for (action_idx, action_config) in visible_actions {
                        let display_name = browser_setting
                            .app_data
                            .actions
                            .get(&action_config.id)
                            .and_then(|a| a.name.clone())
                            .unwrap_or_else(|| action_config.id.clone());

                        let action_wrapper = GtkBox::builder()
                            .orientation(Orientation::Horizontal)
                            .spacing(2)
                            .valign(Align::Center)
                            .build();

                        if is_edit_mode && !action_config.is_visible {
                            action_wrapper.set_opacity(0.4);
                        }

                        // Left Move Button (Edit Mode)
                        if is_edit_mode {
                            let btn_left = Button::builder()
                                .label("◀")
                                .height_request(48)
                                .valign(Align::Center)
                                .build();
                            btn_left.add_css_class("flat");

                            let s_c1 = settings_rc.clone();
                            let c_c1 = container.clone();
                            let app_c1 = app.clone();
                            let u_c1 = url.clone();
                            let p_c1 = settings_path.clone();
                            let path_k1 = path_key.clone();

                            btn_left.connect_clicked(move |_| {
                                if action_idx > 0 {
                                    if let Some(b) = s_c1.borrow_mut().browsers.get_mut(&path_k1) {
                                        b.move_action(action_idx, action_idx - 1);
                                    }
                                    draw_browser_list(&c_c1, &app_c1, &u_c1, &s_c1, &p_c1, true);
                                }
                            });

                            action_wrapper.append(&btn_left);
                        }

                        // Small font label constrained to 2 lines max
                        let action_label = Label::builder()
                            .label(&display_name)
                            .wrap(true)
                            .wrap_mode(WrapMode::WordChar)
                            .justify(Justification::Center)
                            .lines(2)
                            .ellipsize(EllipsizeMode::End)
                            .valign(Align::Center)
                            .build();
                        action_label.add_css_class("caption");

                        // Fixed 48x48 square button layout
                        let btn_action = Button::builder()
                            .child(&action_label)
                            .tooltip_text(&display_name)
                            .width_request(48)
                            .height_request(48)
                            .valign(Align::Center)
                            .build();

                        let app_c = app.clone();
                        let url_c = url.clone();
                        let exec_string = browser_setting
                            .app_data
                            .actions
                            .get(&action_config.id)
                            .and_then(|a| a.exec.clone());
                        let s_c3 = settings_rc.clone();
                        let c_c3 = container.clone();
                        let p_c3 = settings_path.clone();
                        let path_k3 = path_key.clone();
                        let act_id = action_config.id.clone();
                        let app_data = browser_setting.app_data.clone();

                        btn_action.connect_clicked(move |_| {
                            if is_edit_mode {
                                if let Some(b) = s_c3.borrow_mut().browsers.get_mut(&path_k3) {
                                    if let Some(act) = b.actions.iter_mut().find(|a| a.id == act_id) {
                                        act.is_visible = !act.is_visible;
                                    }
                                }
                                draw_browser_list(&c_c3, &app_c, &url_c, &s_c3, &p_c3, true);
                            } else if let Some(exec) = &exec_string {
                                launch_app(&app_c, &url_c, &app_data, Some(exec));
                            }
                        });

                        action_wrapper.append(&btn_action);

                        // Right Move Button (Edit Mode)
                        if is_edit_mode {
                            let btn_right = Button::builder()
                                .label("▶")
                                .height_request(48)
                                .valign(Align::Center)
                                .build();
                            btn_right.add_css_class("flat");

                            let s_c2 = settings_rc.clone();
                            let c_c2 = container.clone();
                            let app_c2 = app.clone();
                            let u_c2 = url.clone();
                            let p_c2 = settings_path.clone();
                            let path_k2 = path_key.clone();

                            btn_right.connect_clicked(move |_| {
                                if action_idx + 1 < actions_len {
                                    if let Some(b) = s_c2.borrow_mut().browsers.get_mut(&path_k2) {
                                        b.move_action(action_idx, action_idx + 1);
                                    }
                                    draw_browser_list(&c_c2, &app_c2, &u_c2, &s_c2, &p_c2, true);
                                }
                            });

                            action_wrapper.append(&btn_right);
                        }

                        actions_box.append(&action_wrapper);
                    }

                    row_box.append(&actions_box);
                }
            }

            container.append(&row_box);
        }
    }
}

fn load_icon(icon_name: &str) -> Image {
    if icon_name.starts_with('/') && Path::new(icon_name).exists() {
        return Image::from_file(icon_name);
    }

    let in_icon_theme = gdk::Display::default()
        .map(|display| gtk4::IconTheme::for_display(&display).has_icon(icon_name))
        .unwrap_or(false);

    // Inside a flatpak icons of other flatpaks (and absolute paths) are usually not visible
    if !in_icon_theme && host::is_flatpak() {
        if let Some(texture) = load_host_icon(icon_name) {
            return Image::from_paintable(Some(&texture));
        }
    }

    Image::from_icon_name(icon_name)
}

fn load_host_icon(icon_name: &str) -> Option<gdk::Texture> {
    thread_local! {
        static CACHE: RefCell<HashMap<String, Option<gdk::Texture>>> = RefCell::new(HashMap::new());
    }

    if let Some(cached) = CACHE.with_borrow(|cache| cache.get(icon_name).cloned()) {
        return cached;
    }

    // Prints the contents of the best matching icon file from the hicolor theme or pixmaps
    let script = r#"
        name=$1; shift
        case $name in /*) exec cat "$name";; esac
        for dir in "$@"; do
            for size in scalable 512x512 256x256 192x192 128x128 96x96 64x64 48x48 32x32 24x24 16x16; do
                for ext in svg png; do
                    file="$dir/icons/hicolor/$size/apps/$name.$ext"
                    [ -f "$file" ] && exec cat "$file"
                done
            done
            for ext in svg png xpm; do
                file="$dir/pixmaps/$name.$ext"
                [ -f "$file" ] && exec cat "$file"
            done
        done
        exit 1"#;

    let dirs: Vec<String> = host::data_dirs().iter().map(|d| d.to_string_lossy().to_string()).collect();
    let mut args = vec![icon_name];
    args.extend(dirs.iter().map(String::as_str));

    let texture = host::capture(script, &args)
        .and_then(|bytes| gdk::Texture::from_bytes(&glib::Bytes::from_owned(bytes)).ok());

    CACHE.with_borrow_mut(|cache| cache.insert(icon_name.to_string(), texture.clone()));
    texture
}

fn launch_app(app: &Application, url: &str, browser: &DesktopApp, custom_exec: Option<&str>) {
    if host::is_flatpak() {
        launch_on_host(url, browser, custom_exec);
        app.quit();
        return;
    }

    let desktop_id = browser
        .path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let desktop_id = desktop_id.as_str();

    let display = gdk::Display::default().expect("Could not get display");
    let launch_context = display.app_launch_context();
    let uris = if url.is_empty() { vec![] } else { vec![url] };

    if let Some(exec) = custom_exec {
        let mut final_exec = exec.to_string();

        if !url.is_empty() {
            let safe_url = format!("'{}'", url.replace("'", "'\\''"));
            if final_exec.contains("%u") || final_exec.contains("%U") {
                final_exec = final_exec.replace("%u", &safe_url).replace("%U", &safe_url);
            } else {
                final_exec = format!("{} {}", final_exec, safe_url);
            }
        } else {
            final_exec = final_exec.replace(" %u", "").replace(" %U", "");
        }

        let app_name = desktop_id.strip_suffix(".desktop").unwrap_or(desktop_id);

        if let Ok(temp_app) = AppInfo::create_from_commandline(
            &final_exec,
            Some(app_name),
            AppInfoCreateFlags::SUPPORTS_STARTUP_NOTIFICATION,
        ) {
            if let Err(e) = temp_app.launch_uris(&[], Some(&launch_context)) {
                eprintln!("Failed to launch custom action: {}", e);
            }
        }
    } else {
        let found_app = AppInfo::all().into_iter().find(|a| {
            a.id().map(|id| id.as_str() == desktop_id).unwrap_or(false)
        });

        if let Some(app_info) = found_app {
            if let Err(e) = app_info.launch_uris(&uris, Some(&launch_context)) {
                eprintln!("Failed to launch {}: {}", desktop_id, e);
            }
        } else {
            eprintln!("Could not find desktop file: {}", desktop_id);
        }
    }

    app.quit();
}

// Host apps can't be started from inside the flatpak sandbox, so run their Exec= on the host
fn launch_on_host(url: &str, browser: &DesktopApp, custom_exec: Option<&str>) {
    let Some(exec) = custom_exec.or(browser.exec.as_deref()) else {
        eprintln!("No Exec= found in {}", browser.path.to_string_lossy());
        return;
    };

    let Some(argv) = crate::browser_manager::desktop_parser::expand_exec(exec, url, browser) else {
        eprintln!("Could not build a command from Exec={}", exec);
        return;
    };

    let cwd = browser
        .work_path
        .as_ref()
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(glib::home_dir);

    if let Err(e) = host::spawn(&argv, &cwd) {
        eprintln!("Failed to launch {:?} on the host: {}", argv, e);
    }
}
