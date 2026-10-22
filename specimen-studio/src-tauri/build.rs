fn main() {
    // The CLI's dev server remains opt-in; plain Cargo embeds fresh assets.
    if std::env::var_os("CARGO_FEATURE_CUSTOM_PROTOCOL").is_some() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("studio workspace");
        for path in [
            "app",
            "components",
            "desktop",
            "hooks",
            "lib",
            "public",
            "native/specimen-kernel",
            "native/specimen-wasm",
            "native/runtime-pins.json",
            "../crates/abi-compute",
            "scripts/build-desktop.py",
            "scripts/build-runtimes.py",
            "package.json",
            "bun.lock",
            "Cargo.lock",
            "vite.desktop.config.ts",
            "tsconfig.json",
        ] {
            println!("cargo:rerun-if-changed={}", root.join(path).display());
        }
        println!("cargo:rerun-if-env-changed=VITE_NATIVE_E2E");
        let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("Cargo output"));
        let profile = out.ancestors().nth(3).expect("Cargo profile directory");
        let status = std::process::Command::new("python3")
            .arg(root.join("scripts/build-desktop.py"))
            .arg("--native-output")
            .arg(profile)
            .current_dir(root)
            .status()
            .expect("desktop assets require Python 3, Bun and the pinned Rust toolchain");
        assert!(status.success(), "desktop asset build failed");
    }
    tauri_build::build()
}
