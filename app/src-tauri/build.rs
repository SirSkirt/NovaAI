use std::{env,fs,path::PathBuf};

fn main() {
    let source=PathBuf::from("resources/nova2-step19000.nnova2");
    println!("cargo:rerun-if-changed={}",source.display());
    let out=PathBuf::from(env::var("OUT_DIR").expect("Cargo OUT_DIR"))
        .join("nova2-bundled.nnova2");
    // Development builds can run with no seed so users can import checkpoints.
    // For a distributable prerelease, CI requires the exact 19,000-step file.
    if source.is_file() {
        fs::copy(&source,&out).expect("copy bundled Nova-2 checkpoint");
    } else {
        fs::write(&out,[]).expect("write empty optional checkpoint marker");
    }
    tauri_build::build();
}
