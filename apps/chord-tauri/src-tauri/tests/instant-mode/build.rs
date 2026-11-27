use std::{env, fs, path::PathBuf};

// Copy exact production items, with no edits, so the portable target exercises the
// same entry point despite Tauri's system-library dependency. Fail on source drift.
fn function(source: &str, signature: &str) -> String {
    let start = source.find(signature).expect("production function exists");
    let opening = start + source[start..].find('{').unwrap();
    let mut depth = 0;
    for (offset, character) in source[opening..].char_indices() {
        match character {
            '{' => depth += 1,
            '}' => depth -= 1,
            _ => {}
        }
        if depth == 0 {
            return source[start..=opening + offset].to_owned();
        }
    }
    panic!("unclosed production function");
}

fn main() {
    let output = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let chord_path = PathBuf::from("../../src/models/chord.rs");
    println!("cargo:rerun-if-changed={}", chord_path.display());
    let chord = fs::read_to_string(chord_path).unwrap();
    let start = chord.find("/// The raw keys").unwrap();
    let end = chord.find("/// A mapping from").unwrap();
    fs::write(
        output.join("models.rs"),
        format!(
            "use serde::Serialize;\nuse typeshare::typeshare;\n{}",
            &chord[start..end]
        ),
    )
    .unwrap();

    let manager_path = PathBuf::from("../../src/app/chord_mode_manager/chord_input_manager.rs");
    println!("cargo:rerun-if-changed={}", manager_path.display());
    let manager = fs::read_to_string(manager_path).unwrap();
    let mut source = "impl ChordInputManager {\n".to_string();
    for signature in [
        "pub fn reset(",
        "pub fn begin(",
        "pub fn handle_key_event(",
        "fn should_execute_key_buffer_on_release(",
        "fn next_key_buffer_for_key_press(",
        "fn expand_typed_reference(",
        "fn handle_key_press(",
        "    fn close_block(",
        "fn should_clear_key_buffer_on_shifted_press(",
        "fn clear_key_buffer(",
        "fn clear_loose_keys(",
        "fn edit_buffer(",
        "fn set_executed_event(",
        "fn fail_input(",
    ] {
        source.push_str(&function(&manager, signature));
        source.push('\n');
    }
    source.push_str("}\n");
    for signature in [
        "\nfn close_block(",
        "\nfn rebase_blocks(",
        "fn track_key_event(",
        "fn should_handle_held_key_event(",
        "fn tab_toggles_hint_panel(",
        "fn format_keys(",
    ] {
        source.push_str(&function(&manager, signature));
        source.push('\n');
    }
    let sequence_path = PathBuf::from("../../src/app/chord_mode_manager/chord_sequence.rs");
    println!("cargo:rerun-if-changed={}", sequence_path.display());
    let sequence = fs::read_to_string(sequence_path).unwrap();
    source.push_str("impl Sequence {\n");
    source.push_str(&function(&sequence, "pub fn edit_quotes("));
    source.push_str("\n}\n");
    fs::write(output.join("manager.rs"), source).unwrap();
}
