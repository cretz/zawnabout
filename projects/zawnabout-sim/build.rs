//! Compiles the world KDL into Rust data, so the world ships inside the binary.

use std::path::Path;

fn main() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../zawnabout-world/first");
    println!("cargo:rerun-if-changed={}", dir.display());
    // The gameplay file is the play crate's, not the world's.
    for entry in std::fs::read_dir(&dir).expect("world dir") {
        let path = entry.expect("entry").path();
        if !path.ends_with("play.kdl") {
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }
    let rust = zawnabout_worldc::compile(&dir).unwrap_or_else(|e| {
        // One line per problem, each with its file and line.
        for line in e.lines() {
            println!("cargo:warning={line}");
        }
        panic!("world has errors:\n{e}");
    });
    // Only touch the file when it changes, so rustc's incremental cache stays warm.
    let out = Path::new(&std::env::var("OUT_DIR").expect("OUT_DIR")).join("world.rs");
    if std::fs::read_to_string(&out).ok().as_deref() != Some(rust.as_str()) {
        std::fs::write(out, rust).expect("write world.rs");
    }
}
