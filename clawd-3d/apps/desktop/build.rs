use std::{env, fs, path::Path};

fn main() {
    // The move names for the menu and for checking messages, from the moves clawd-engine.js exports for the Rust port.
    let json = "../../rust/clawd-core/moves.json";
    println!("cargo:rerun-if-changed={json}");
    let data: serde_json::Value = serde_json::from_str(&fs::read_to_string(json).expect("read moves.json")).unwrap();
    let names: Vec<String> = data["moves"]
        .as_array()
        .expect("moves.json has moves")
        .iter()
        .map(|m| format!("{:?}", m["name"].as_str().expect("a move has a name")))
        .collect();
    let out = Path::new(&env::var("OUT_DIR").unwrap()).join("moves.rs");
    fs::write(out, format!("const MOVES: &[&str] = &[{}];\n", names.join(", "))).unwrap();
    tauri_build::build()
}
