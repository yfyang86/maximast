// Capture the rustc version at build time so the plugin ABI string can
// encode it. A plugin built with a different toolchain produces a different
// ABI string, which the host rejects (the Rust ABI is not stable across
// compiler versions). Both the host and any path-dependent plugin compile
// this same crate, so both observe the same value.
use std::process::Command;

fn main() {
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
    let version = Command::new(rustc)
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=MAXIMA_RUSTC_VERSION={}", version);
}
