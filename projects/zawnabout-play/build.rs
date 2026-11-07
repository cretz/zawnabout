//! Compiles the gameplay KDL into Rust data, so it ships inside the binary.

use std::path::Path;

fn main() {
    let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../world/first/play.kdl");
    println!("cargo:rerun-if-changed={}", file.display());
    let rust = zawnabout_worldc::play::compile(&file).unwrap_or_else(|e| {
        for line in e.lines() {
            println!("cargo:warning={line}");
        }
        panic!("play.kdl has errors:\n{e}");
    });
    // Only touch the file when it changes, so rustc's incremental cache stays warm.
    let out = Path::new(&std::env::var("OUT_DIR").expect("OUT_DIR")).join("play.rs");
    if std::fs::read_to_string(&out).ok().as_deref() != Some(rust.as_str()) {
        std::fs::write(out, rust).expect("write play.rs");
    }
}
