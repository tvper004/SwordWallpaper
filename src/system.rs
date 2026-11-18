#[cfg(target_os = "linux")]
use zbus::Connection;

/// Queries the Feral GameMode system D-Bus service.
/// Returns Some(true) if a game has registered with GameMode,
/// Some(false) if GameMode is running and idle,
/// or None if GameMode is not available.
#[cfg(target_os = "linux")]
pub async fn check_gamemode() -> Option<bool> {
    let conn = tokio::time::timeout(
        std::time::Duration::from_millis(300),
        Connection::system(),
    )
    .await
    .ok()?
    .ok()?;

    let proxy = zbus::Proxy::new(
        &conn,
        "com.feralinteractive.GameMode",
        "/com/feralinteractive/GameMode",
        "com.feralinteractive.GameMode",
    )
    .await
    .ok()?
    .ok()?;

    let status: i32 = tokio::time::timeout(
        std::time::Duration::from_millis(300),
        proxy.call("QueryStatus", &()),
    )
    .await
    .ok()?
    .ok()?;

    Some(status > 0)
}

#[cfg(not(target_os = "linux"))]
pub async fn check_gamemode() -> Option<bool> {
    None
}

/// Queries UPower over system D-Bus to determine if device is running on battery.
/// Returns Some(true) if on battery, Some(false) if on AC power,
/// or None if UPower is not reachable.
#[cfg(target_os = "linux")]
pub async fn check_on_battery() -> Option<bool> {
    let conn = tokio::time::timeout(
        std::time::Duration::from_millis(300),
        Connection::system(),
    )
    .await
    .ok()?
    .ok()?;

    let proxy = zbus::Proxy::new(
        &conn,
        "org.freedesktop.UPower",
        "/org/freedesktop/UPower",
        "org.freedesktop.UPower",
    )
    .await
    .ok()?
    .ok()?;

    let on_battery: bool = tokio::time::timeout(
        std::time::Duration::from_millis(300),
        proxy.get_property("OnBattery"),
    )
    .await
    .ok()?
    .ok()?;

    Some(on_battery)
}

#[cfg(target_os = "macos")]
pub async fn check_on_battery() -> Option<bool> {
    tokio::task::spawn_blocking(|| {
        if let Ok(output) = std::process::Command::new("pmset").arg("-g").arg("batt").output() {
            let s = String::from_utf8_lossy(&output.stdout);
            if s.contains("Battery Power") {
                return Some(true);
            }
            if s.contains("AC Power") {
                return Some(false);
            }
        }
        None
    })
    .await
    .ok()
    .flatten()
}

#[cfg(all(not(target_os = "linux"), not(target_os = "macos")))]
pub async fn check_on_battery() -> Option<bool> {
    None
}

pub fn is_heavy_process_cmdline(cmdline: &str) -> bool {
    let lower = cmdline.to_lowercase();
    if lower.contains("protonvpn") {
        return false;
    }
    if lower.contains("gamescope")
        || lower.contains("proton")
        || lower.contains("steam_app")
        || lower.contains("heroic --wine")
        || lower.contains("wine64-preloader")
        || lower.contains("lutris-wrapper")
        || lower.contains("steam_osx")
        || lower.contains("crossover")
        || lower.contains("retroarch")
        || lower.contains("openemu")
        || lower.contains("rpcs3")
        || lower.contains("ryujinx")
    {
        return true;
    }
    false
}

pub async fn check_if_fullscreen_game_active() -> bool {
    // 1. Fast check: GameMode D-Bus (Linux)
    if let Some(in_game) = check_gamemode().await {
        if in_game {
            return true;
        }
    }

    // 2. Check running processes
    #[cfg(target_os = "macos")]
    {
        tokio::task::spawn_blocking(|| {
            if let Ok(output) = std::process::Command::new("ps").args(&["-eo", "comm="]).output() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    if is_heavy_process_cmdline(line) {
                        return true;
                    }
                }
            }
            false
        })
        .await
        .unwrap_or(false)
    }

    #[cfg(not(target_os = "macos"))]
    {
        tokio::task::spawn_blocking(|| {
            if let Ok(entries) = std::fs::read_dir("/proc") {
                for entry in entries.filter_map(|e| e.ok()) {
                    let is_pid = entry
                        .file_name()
                        .to_str()
                        .map_or(false, |s| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()));

                    if is_pid {
                        if let Ok(cmdline) = std::fs::read_to_string(entry.path().join("cmdline")) {
                            if is_heavy_process_cmdline(&cmdline) {
                                return true;
                            }
                        }
                    }
                }
            }
            false
        })
        .await
        .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_heavy_process_cmdline() {
        assert!(is_heavy_process_cmdline("/usr/bin/gamescope -W 1920"));
        assert!(is_heavy_process_cmdline("proton run game.exe"));
        assert!(is_heavy_process_cmdline("heroic --wine game.exe"));
        assert!(is_heavy_process_cmdline("steam_app_12345"));
        assert!(is_heavy_process_cmdline("wine64-preloader app.exe"));
        assert!(is_heavy_process_cmdline("lutris-wrapper game"));
        assert!(is_heavy_process_cmdline("/Applications/Steam.app/Contents/MacOS/steam_osx"));
        assert!(is_heavy_process_cmdline("CrossOver"));
        assert!(!is_heavy_process_cmdline("/usr/bin/bash"));
        assert!(!is_heavy_process_cmdline("cosmic-panel"));
        assert!(!is_heavy_process_cmdline("/usr/bin/protonvpn-app"));
        assert!(!is_heavy_process_cmdline("/opt/heroic/heroic"));
        assert!(!is_heavy_process_cmdline(""));
    }
}
