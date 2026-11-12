// Keep the Lua catalog as the only registry an artist needs to edit.
use std::{env, fs, path::PathBuf};

fn main() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/mods/pings/audio");
    println!("cargo:rerun-if-changed={}", dir.display());
    let mut files: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "wav"))
        .collect();
    files.sort();
    let mut source = String::from("&[\n");
    for file in files {
        let name = file.file_stem().unwrap().to_str().unwrap();
        source.push_str(&format!("({name:?}, include_bytes!({file:?})),\n"));
    }
    source.push(']');
    fs::write(
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("ping_audio.rs"),
        source,
    )
    .unwrap();
}
