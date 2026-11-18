mod app;
mod cli;
mod config;
mod engine;
mod i18n;
mod scanner;
mod theme;
mod tray;
mod online;
mod system;
mod ui;

fn ensure_wayland_display() {
    #[cfg(target_os = "linux")]
    if std::env::var_os("WAYLAND_DISPLAY").is_none() {
        if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
            for name in &["wayland-1", "wayland-0", "wayland-2"] {
                let p = std::path::Path::new(&runtime_dir).join(name);
                if p.exists() {
                    std::env::set_var("WAYLAND_DISPLAY", name);
                    return;
                }
            }
            if let Ok(entries) = std::fs::read_dir(&runtime_dir) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let fname = entry.file_name();
                    let s = fname.to_string_lossy();
                    if s.starts_with("wayland-") && !s.ends_with(".lock") {
                        std::env::set_var("WAYLAND_DISPLAY", s.as_ref());
                        return;
                    }
                }
            }
        }
    }
}

fn ensure_environment() {
    ensure_wayland_display();
    // Ensure homebrew and local bin directories are in PATH so helper utilities (aura-bg, mpvpaper, ffmpeg) are always found
    let mut extra_paths = Vec::new();
    if let Ok(home) = std::env::var("HOME") {
        extra_paths.push(format!("{}/.local/bin", home));
    }
    #[cfg(target_os = "macos")]
    {
        extra_paths.push("/opt/homebrew/bin".to_string());
        extra_paths.push("/usr/local/bin".to_string());
    }

    if let Ok(path) = std::env::var("PATH") {
        let mut new_path = path.clone();
        for p in extra_paths {
            if !path.split(':').any(|item| item == p) && std::path::Path::new(&p).exists() {
                new_path = format!("{}:{}", p, new_path);
            }
        }
        std::env::set_var("PATH", new_path);
    }
}

fn main() -> cosmic::iced::Result {
    ensure_environment();
    let args: Vec<String> = std::env::args().collect();
    if cli::is_pure_cli(&args) {
        cli::handle_pure_cli(&args);
        return Ok(());
    }

    let (hidden, action, action_args) = cli::parse_flags(&args);
    let initial_cfg = config::Config::load();
    let is_honeycomb = initial_cfg.switcher_style == config::SwitcherStyle::Honeycomb;
    if let Some(ref act) = action {
        if act != "switcher" {
            println!("🌌 SwordW: Procesando acción '{}'...", act);
            cli::handle_action_cli(act, &action_args);
            return Ok(());
        }
        if cli::try_send_gui_ipc("switcher") {
            println!("🌌 SwordW: Switcher activado.");
            return Ok(());
        }
        println!("🌌 SwordW: Procesando acción '{}'...", act);
    } else if cli::try_send_gui_ipc(if is_honeycomb { "switcher" } else { "show" }) {
        println!("🌌 SwordW: Ventana activada.");
        return Ok(());
    }
    let flags = app::AuraFlags {
        hidden,
        action,
        args: action_args,
    };

    #[cfg(target_os = "macos")]
    if !hidden {
        tray::activate_mac_app();
    }

    #[cfg(target_os = "macos")]
    let initial_dark = tray::macos_is_dark_appearance();
    #[cfg(target_os = "macos")]
    let initial_theme = ui::sword_theme(initial_dark);
    #[cfg(not(target_os = "macos"))]
    let initial_theme = cosmic::theme::system_preference();

    let mut settings = cosmic::app::Settings::default()
        .theme(initial_theme)
        .size(cosmic::iced::Size::new(1500.0, 900.0))
        .size_limits(
            cosmic::iced::Limits::NONE
                .min_width(1180.0)
                .min_height(640.0),
        )
        .exit_on_close(false)
        .transparent(true)
        .is_daemon(false);

    if hidden || is_honeycomb {
        settings = settings.no_main_window(true);
    }

    cosmic::app::run_single_instance::<app::AuraApp>(settings, flags)
}
