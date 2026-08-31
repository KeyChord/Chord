fn main() {
    // Packages keep prebuilt Node-API modules under `target/<triple>/...`; the `chord`
    // module's `resolveNativeModulePath` needs the app's triple to pick the right subtree.
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

    // rbun links `libbun_embed.dylib` dynamically. Dev builds find it where
    // the rbun checkout built it; the app bundle ships it in
    // `Contents/Frameworks` (`bundle.macOS.frameworks` in tauri.conf.json).
    if let Ok(lib_dir) = std::env::var("DEP_BUN_EMBED_LIB_DIR") {
        // Not `-bins`: the test harness links the library too.
        println!("cargo:rustc-link-arg=-Wl,-rpath,{lib_dir}");
    }
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo:rustc-link-arg-bins=-Wl,-rpath,@executable_path/../Frameworks");
    }

    tauri_build::build();
}
