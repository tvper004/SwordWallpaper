use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::sync::Mutex as TokioMutex;

#[cfg(target_os = "linux")]
use ksni::menu::{CheckmarkItem, MenuItem, StandardItem};
#[cfg(target_os = "linux")]
use ksni::{Handle, ToolTip, Tray};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum TrayAction {
    Activate,
    ShowApp,
    HideApp,
    ToggleAutostart,
    OpenSwitcher,
    TogglePause,
    NextWallpaper,
    StopWallpaper,
    QuitApp,
}

#[derive(Clone)]
#[allow(dead_code)]
pub struct AuraTray {
    pub tx: UnboundedSender<TrayAction>,
    pub current_name: Arc<Mutex<String>>,
    pub is_paused: Arc<Mutex<bool>>,
    pub has_wallpaper: Arc<Mutex<bool>>,
    pub autostart: Arc<Mutex<bool>>,
    pub language: Arc<Mutex<crate::i18n::Language>>,
}

#[cfg(target_os = "linux")]
fn get_tray_icon_pixmap() -> Vec<ksni::Icon> {
    let icon_bytes = include_bytes!("../resources/icons/aura_tray_translucent_44.png");
    if let Ok(img) = image::load_from_memory(icon_bytes) {
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        let mut argb = Vec::with_capacity((width * height * 4) as usize);
        for pixel in rgba.pixels() {
            // ksni expects ARGB32 network byte order (Alpha, Red, Green, Blue)
            argb.push(pixel[3]);
            argb.push(pixel[0]);
            argb.push(pixel[1]);
            argb.push(pixel[2]);
        }
        vec![ksni::Icon {
            width: width as i32,
            height: height as i32,
            data: argb,
        }]
    } else {
        Vec::new()
    }
}

#[cfg(target_os = "linux")]
impl Tray for AuraTray {
    fn id(&self) -> String {
        "io.github.antwny.aura".into()
    }

    fn title(&self) -> String {
        "Sword Wallpaper".into()
    }

    fn icon_name(&self) -> String {
        "io.github.antwny.aura".into()
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        get_tray_icon_pixmap()
    }

    fn tool_tip(&self) -> ToolTip {
        let lang = *self.language.lock().unwrap();
        let name = self.current_name.lock().unwrap().clone();
        let desc = if name.is_empty() {
            lang.tray_tooltip_idle().to_string()
        } else {
            lang.tray_tooltip_playing(&name)
        };
        ToolTip {
            title: "Sword Wallpaper".into(),
            description: desc,
            icon_name: "io.github.antwny.aura".into(),
            icon_pixmap: Vec::new(),
        }
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        let _ = self.tx.send(TrayAction::Activate);
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let lang = *self.language.lock().unwrap();
        let autostart_val = *self.autostart.lock().unwrap();

        vec![
            CheckmarkItem {
                label: lang.tray_autostart().into(),
                checked: autostart_val,
                activate: Box::new(|tray: &mut AuraTray| {
                    let _ = tray.tx.send(TrayAction::ToggleAutostart);
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: lang.tray_show_window().into(),
                activate: Box::new(|tray: &mut AuraTray| {
                    let _ = tray.tx.send(TrayAction::ShowApp);
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: lang.tray_close().into(),
                activate: Box::new(|tray: &mut AuraTray| {
                    let _ = tray.tx.send(TrayAction::QuitApp);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}

pub struct TrayController {
    #[allow(dead_code)]
    pub rx: Arc<TokioMutex<UnboundedReceiver<TrayAction>>>,
    pub current_name: Arc<Mutex<String>>,
    pub is_paused: Arc<Mutex<bool>>,
    pub has_wallpaper: Arc<Mutex<bool>>,
    pub autostart: Arc<Mutex<bool>>,
    pub language: Arc<Mutex<crate::i18n::Language>>,
    #[cfg(target_os = "linux")]
    pub handle: Arc<Mutex<Option<Handle<AuraTray>>>>,
}

static TRAY_RX: std::sync::OnceLock<Arc<TokioMutex<UnboundedReceiver<TrayAction>>>> = std::sync::OnceLock::new();
static TRAY_TX: std::sync::OnceLock<UnboundedSender<TrayAction>> = std::sync::OnceLock::new();

pub fn get_tray_rx() -> Option<Arc<TokioMutex<UnboundedReceiver<TrayAction>>>> {
    TRAY_RX.get().cloned()
}

pub fn get_tray_tx() -> Option<UnboundedSender<TrayAction>> {
    TRAY_TX.get().cloned()
}

#[cfg(target_os = "macos")]
mod ffi {
    extern "C" {
        pub fn setup_mac_tray(
            icon_bytes: *const u8,
            icon_len: usize,
            autostart_initial: bool,
            title_autostart: *const std::ffi::c_char,
            title_show: *const std::ffi::c_char,
            title_close: *const std::ffi::c_char,
            callback: extern "C" fn(action_code: i32),
        );
        pub fn update_mac_tray_autostart(autostart: bool);
        pub fn update_mac_tray_labels(
            title_autostart: *const std::ffi::c_char,
            title_show: *const std::ffi::c_char,
            title_close: *const std::ffi::c_char,
        );
        pub fn update_mac_tray_show_label(title_show: *const std::ffi::c_char);
        pub fn hide_mac_main_window();
        pub fn show_mac_main_window();
        pub fn hide_mac_switcher_window();
        pub fn show_mac_switcher_window();
        pub fn minimize_mac_main_window();
        pub fn zoom_mac_main_window();
        pub fn macos_set_window_transparent();
        pub fn activate_mac_app();
        pub fn ensure_mac_window_capabilities();
        pub fn macos_get_screen_size(width: *mut f32, height: *mut f32);
        pub fn sword_wallpaper_macos_is_dark_appearance() -> i32;
    }
}

#[cfg(target_os = "macos")]
pub fn macos_is_dark_appearance() -> bool {
    unsafe { ffi::sword_wallpaper_macos_is_dark_appearance() != 0 }
}

#[cfg(target_os = "macos")]
pub fn get_mac_screen_size() -> (f32, f32) {
    let mut w: f32 = 1920.0;
    let mut h: f32 = 1080.0;
    unsafe {
        ffi::macos_get_screen_size(&mut w as *mut f32, &mut h as *mut f32);
    }
    (w, h)
}

#[cfg(target_os = "macos")]
#[allow(dead_code)]
pub fn ensure_mac_window_capabilities() {
    unsafe {
        ffi::ensure_mac_window_capabilities();
    }
}

#[cfg(target_os = "macos")]
#[allow(dead_code)]
pub fn hide_mac_window() {
    unsafe {
        ffi::hide_mac_main_window();
    }
}

#[cfg(target_os = "macos")]
#[allow(dead_code)]
pub fn show_mac_window() {
    unsafe {
        ffi::show_mac_main_window();
    }
}

#[cfg(target_os = "macos")]
#[allow(dead_code)]
pub fn hide_mac_switcher() {
    unsafe {
        ffi::hide_mac_switcher_window();
    }
}

#[cfg(target_os = "macos")]
#[allow(dead_code)]
pub fn show_mac_switcher() {
    unsafe {
        ffi::show_mac_switcher_window();
    }
}

#[cfg(target_os = "macos")]
#[allow(dead_code)]
pub fn minimize_mac_window() {
    unsafe {
        ffi::minimize_mac_main_window();
    }
}

#[cfg(target_os = "macos")]
#[allow(dead_code)]
pub fn zoom_mac_window() {
    unsafe {
        ffi::zoom_mac_main_window();
    }
}

#[cfg(target_os = "macos")]
#[allow(dead_code)]
pub fn set_mac_window_transparent() {
    unsafe {
        ffi::macos_set_window_transparent();
    }
}

#[cfg(target_os = "macos")]
pub fn activate_mac_app() {
    unsafe {
        ffi::activate_mac_app();
    }
}

#[cfg(target_os = "macos")]
pub fn update_mac_tray_window_label(is_window_open: bool) {
    let label = if is_window_open { "Cerrar ventana" } else { "Mostrar ventana" };
    if let Ok(c_label) = std::ffi::CString::new(label) {
        unsafe {
            ffi::update_mac_tray_show_label(c_label.as_ptr());
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn update_mac_tray_window_label(_is_window_open: bool) {}

#[cfg(target_os = "macos")]
extern "C" fn mac_tray_callback(action_code: i32) {
    if let Some(tx) = get_tray_tx() {
        let action = match action_code {
            1 => TrayAction::ToggleAutostart,
            2 => TrayAction::ShowApp,
            3 => TrayAction::QuitApp,
            4 => TrayAction::HideApp,
            _ => return,
        };
        let _ = tx.send(action);
    }
}

impl TrayController {
    pub fn new(lang: crate::i18n::Language, autostart: bool) -> (Self, AuraTray) {
        let (tx, rx) = unbounded_channel();
        let _ = TRAY_TX.set(tx.clone());
        let rx = Arc::new(TokioMutex::new(rx));
        let _ = TRAY_RX.set(rx.clone());
        let current_name = Arc::new(Mutex::new(String::new()));
        let is_paused = Arc::new(Mutex::new(false));
        let has_wallpaper = Arc::new(Mutex::new(false));
        let autostart_state = Arc::new(Mutex::new(autostart));
        let language = Arc::new(Mutex::new(lang));
        #[cfg(target_os = "linux")]
        let handle = Arc::new(Mutex::new(None));

        let tray = AuraTray {
            tx,
            current_name: current_name.clone(),
            is_paused: is_paused.clone(),
            has_wallpaper: has_wallpaper.clone(),
            autostart: autostart_state.clone(),
            language: language.clone(),
        };

        let controller = Self {
            rx,
            current_name,
            is_paused,
            has_wallpaper,
            autostart: autostart_state,
            language,
            #[cfg(target_os = "linux")]
            handle,
        };

        (controller, tray)
    }

    pub fn set_autostart(&self, autostart: bool) {
        *self.autostart.lock().unwrap() = autostart;
        #[cfg(target_os = "linux")]
        {
            let handle_opt = self.handle.lock().unwrap().clone();
            if let Some(handle) = handle_opt {
                tokio::spawn(async move {
                    let _ = handle.update(|_| {}).await;
                });
            }
        }
        #[cfg(target_os = "macos")]
        {
            unsafe {
                ffi::update_mac_tray_autostart(autostart);
            }
        }
    }

    pub fn set_language(&self, lang: crate::i18n::Language) {
        *self.language.lock().unwrap() = lang;
        #[cfg(target_os = "linux")]
        {
            let handle_opt = self.handle.lock().unwrap().clone();
            if let Some(handle) = handle_opt {
                tokio::spawn(async move {
                    let _ = handle.update(|_| {}).await;
                });
            }
        }
        #[cfg(target_os = "macos")]
        {
            let autostart_title = std::ffi::CString::new(lang.tray_autostart()).unwrap_or_default();
            let show_title = std::ffi::CString::new(lang.tray_show_window()).unwrap_or_default();
            let close_title = std::ffi::CString::new(lang.tray_close()).unwrap_or_default();
            unsafe {
                ffi::update_mac_tray_labels(autostart_title.as_ptr(), show_title.as_ptr(), close_title.as_ptr());
            }
        }
    }

    pub fn update_state(&self, name: String, paused: bool, has_wall: bool) {
        *self.current_name.lock().unwrap() = name;
        *self.is_paused.lock().unwrap() = paused;
        *self.has_wallpaper.lock().unwrap() = has_wall;

        #[cfg(target_os = "linux")]
        {
            let handle_opt = self.handle.lock().unwrap().clone();
            if let Some(handle) = handle_opt {
                tokio::spawn(async move {
                    let _ = handle.update(|_| {}).await;
                });
            }
        }
    }

    pub fn spawn_service(&self, tray: AuraTray) {
        #[cfg(target_os = "linux")]
        {
            let handle_store = self.handle.clone();
            tokio::spawn(async move {
                let is_sandboxed = std::path::Path::new("/.flatpak-info").exists()
                    || std::env::var_os("FLATPAK_ID").is_some()
                    || std::env::var_os("SNAP").is_some();

                for attempt in 1..=15 {
                    let builder = tray.clone()
                        .disable_dbus_name(is_sandboxed)
                        .assume_sni_available(true);
                    match builder.spawn().await {
                        Ok(handle) => {
                            *handle_store.lock().unwrap() = Some(handle);
                            return;
                        }
                        Err(e) => {
                            if attempt == 15 {
                                eprintln!("[Aura Tray] Error al registrar StatusNotifierItem tras 15 intentos: {:?}", e);
                            } else {
                                tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
                            }
                        }
                    }
                }
            });
        }

        #[cfg(target_os = "macos")]
        {
            let _ = tray;
            let lang = *self.language.lock().unwrap();
            let autostart_init = *self.autostart.lock().unwrap();
            let autostart_title = std::ffi::CString::new(lang.tray_autostart()).unwrap_or_default();
            let show_title = std::ffi::CString::new(lang.tray_show_window()).unwrap_or_default();
            let close_title = std::ffi::CString::new(lang.tray_close()).unwrap_or_default();

            let icon_bytes = include_bytes!("../resources/icons/aura_tray_sword_44.png");

            unsafe {
                ffi::setup_mac_tray(
                    icon_bytes.as_ptr(),
                    icon_bytes.len(),
                    autostart_init,
                    autostart_title.as_ptr(),
                    show_title.as_ptr(),
                    close_title.as_ptr(),
                    mac_tray_callback,
                );
            }

            if let Ok(mut sig) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::user_defined1()) {
                tokio::spawn(async move {
                    while sig.recv().await.is_some() {
                        if let Some(tx) = get_tray_tx() {
                            let _ = tx.send(TrayAction::ShowApp);
                        }
                    }
                });
            }
        }

        #[cfg(all(not(target_os = "linux"), not(target_os = "macos")))]
        {
            let _ = tray;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tray_controller_creation_and_autostart_toggle() {
        let (controller, _tray) = TrayController::new(crate::i18n::Language::Es, true);
        assert_eq!(*controller.autostart.lock().unwrap(), true);

        controller.set_autostart(false);
        assert_eq!(*controller.autostart.lock().unwrap(), false);

        controller.set_autostart(true);
        assert_eq!(*controller.autostart.lock().unwrap(), true);
    }

    #[test]
    fn test_tray_actions_enum() {
        let act = TrayAction::ShowApp;
        assert_eq!(act, TrayAction::ShowApp);
        let act_auto = TrayAction::ToggleAutostart;
        assert_eq!(act_auto, TrayAction::ToggleAutostart);
        let act_quit = TrayAction::QuitApp;
        assert_eq!(act_quit, TrayAction::QuitApp);
    }
}

