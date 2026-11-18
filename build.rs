fn main() {
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rerun-if-changed=src/engine/macos_bg.swift");
        let swift_src = "src/engine/macos_bg.swift";
        if std::path::Path::new(swift_src).exists() {
            if let Ok(out_dir) = std::env::var("OUT_DIR") {
                let out_path = std::path::PathBuf::from(&out_dir);
                if let Some(target_profile_dir) = out_path.ancestors().nth(3) {
                    let dest = target_profile_dir.join("aura-bg");
                    let status = std::process::Command::new("swiftc")
                        .env("MACOSX_DEPLOYMENT_TARGET", "13.0")
                        .args(&["-O", swift_src, "-o"])
                        .arg(&dest)
                        .status().expect("Could not launch the Swift compiler");
                    assert!(status.success(), "Could not compile aura-bg; refusing to package an outdated helper");
                }
            }
        }

        println!("cargo:rerun-if-changed=src/tray_macos.m");
        cc::Build::new()
            .file("src/tray_macos.m")
            .flag("-fobjc-arc")
            .compile("aura_tray_macos");
        println!("cargo:rustc-link-lib=framework=Cocoa");
    }
}
