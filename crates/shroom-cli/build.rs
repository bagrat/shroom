//! The command says shroom's version: the bundle's own, from the app's tauri.conf.json (the
//! one tools/bump-version.sh moves, and the one the bundle's Info.plist carries), never this
//! crate's, which nothing moves. Read as bump-version.sh reads it: the line `  "version": "…",`.

fn main() {
    let conf = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../app/src-tauri/tauri.conf.json");
    println!("cargo:rerun-if-changed={}", conf.display());
    let text = std::fs::read_to_string(&conf).unwrap_or_else(|e| panic!("{}: {e}", conf.display()));
    let version = text
        .lines()
        .find_map(|l| l.strip_prefix("  \"version\": \"").and_then(|r| r.strip_suffix("\",")))
        .unwrap_or_else(|| panic!("{}: no version line", conf.display()));
    println!("cargo:rustc-env=SHROOM_VERSION={version}");
}
