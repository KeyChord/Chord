fn main() {
    bundle_core_chords();
    // Packages keep prebuilt Node-API modules under `target/<triple>/...`; the `chord`
    // `import.meta.chord.resolveNative` needs the app's triple to pick the right subtree.
    println!(
        "cargo:rustc-env=CHORD_TARGET_TRIPLE={}",
        std::env::var("TARGET").expect("cargo sets TARGET")
    );
    println!("cargo:rerun-if-changed=../data/*");

    // rbun links `libbun_embed.dylib` dynamically. Development builds (`tauri dev`,
    // `cargo test`, plain `cargo build`) run from `target/` and find it where rbun
    // built or CI downloaded it. `tauri build` ships it in `Contents/Frameworks`
    // (`bundle.macOS.frameworks` in tauri.conf.json), and that binary must not look
    // anywhere else: Chord has library validation off (Entitlements.plist), so a
    // directory outside the bundle on its rpath, such as CI's
    // /Users/runner/work/_temp/bun-embed, would load whatever someone puts there.
    // XProtect refuses to open such an app ("Apple could not verify Chord is free of
    // malware"), notarized or not.
    if tauri_build::is_dev() {
        if let Ok(lib_dir) = std::env::var("DEP_BUN_EMBED_LIB_DIR") {
            // Not `-bins`: the test harness links the library too.
            println!("cargo:rustc-link-arg=-Wl,-rpath,{lib_dir}");
        }
    }
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo:rustc-link-arg-bins=-Wl,-rpath,@executable_path/../Frameworks");
    }

    tauri_build::build();
}

fn bundle_core_chords() {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let relative = "chords/com/leonsilicon/chord/macos.toml";
    let canonical = manifest
        .join("../../../../chords/@keychord/chords-chord")
        .join(relative);
    let bundled = manifest.join("../data/chords-chord").join(relative);
    println!("cargo:rerun-if-changed={}", canonical.display());
    println!("cargo:rerun-if-changed={}", bundled.display());
    let source = if canonical.is_file() {
        canonical
    } else {
        bundled
    };
    let contents = std::fs::read(&source).expect("the core Chord bindings must be bundled");
    let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("cargo sets OUT_DIR"));
    std::fs::write(output.join("chords-chord.macos.toml"), contents)
        .expect("write bundled Chord bindings");
    println!("cargo:rustc-env=CHORD_CORE_SOURCE={}", source.display());
}
