fn main() {
    // Native handler artifacts live under `target/<triple>/...` in packages; the app needs its
    // own triple at runtime to pick the right subtree.
    println!(
        "cargo:rustc-env=CHORD_TARGET_TRIPLE={}",
        std::env::var("TARGET").expect("cargo sets TARGET")
    );
    println!("cargo:rerun-if-changed=native/hid.c");
    println!("cargo:rerun-if-changed=native/caps.c");
    println!("cargo:rerun-if-changed=../data/*");

    cc::Build::new()
        .file("native/hid.c")
        .file("native/caps.c")
        .compile("hid_caps");

    tauri_build::build();
}
