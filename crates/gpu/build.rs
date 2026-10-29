// Embeds all effect shaders from /shaders into the binary.
use std::{env, fs, path::Path};

fn main() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../shaders");
    println!("cargo:rerun-if-changed={}", dir.display());
    let mut entries: Vec<_> = fs::read_dir(&dir).unwrap().flatten().map(|e| e.path()).collect();
    entries.sort();
    let mut out = String::from("pub static SHADERS: &[(&str, &str)] = &[\n");
    for p in entries {
        if p.extension().and_then(|e| e.to_str()) != Some("wgsl") {
            continue;
        }
        println!("cargo:rerun-if-changed={}", p.display());
        let name = p.file_stem().unwrap().to_str().unwrap();
        out.push_str(&format!("    ({name:?}, include_str!({:?})),\n", p.canonicalize().unwrap()));
    }
    out.push_str("];\n");
    fs::write(Path::new(&env::var("OUT_DIR").unwrap()).join("shaders.rs"), out).unwrap();
}
