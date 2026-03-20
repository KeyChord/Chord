fn main() {
    println!("cargo:rerun-if-changed=native/hid.c");
    println!("cargo:rerun-if-changed=native/caps.c");
    println!("cargo:rerun-if-changed=../data/global-hotkey-pool.jsonc");

    cc::Build::new()
        .file("native/hid.c")
        .file("native/caps.c")
        .compile("hid_caps");

    tauri_build::build()
}
