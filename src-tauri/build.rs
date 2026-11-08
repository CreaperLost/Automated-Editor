use std::{env, path::PathBuf, process::Command};

fn main() {
    let skip_swift = env::var("AEROEDITS_SKIP_SWIFT").is_ok();
    println!("cargo:rerun-if-env-changed=AEROEDITS_SKIP_SWIFT");
    println!("cargo:rustc-check-cfg=cfg(stub_swift_ffi)");
    if skip_swift {
        println!("cargo:rustc-cfg=stub_swift_ffi");
    }
    if !skip_swift && env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        build_macos_editor_bridge();
    }
    // llama.cpp's Vulkan backend links the Vulkan loader. Loaded on first use instead of at
    // start, the app still opens on a PC without one; `ai::local` checks for it first.
    if env::var_os("CARGO_FEATURE_LOCAL_LLM").is_some()
        && env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
    {
        println!("cargo:rustc-link-arg=/DELAYLOAD:vulkan-1.dll");
        println!("cargo:rustc-link-lib=delayimp");
    }

    embed_images();

    #[cfg(feature = "tauri-app")]
    tauri_build::build();
}

/// Lists every picture of the image library (`assets/images`, beside its `catalog.json`) for
/// `image_library` to build in, as `(path in the library, bytes)`.
fn embed_images() {
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("assets/images");
    println!("cargo:rerun-if-changed={}", root.display());
    let mut files = Vec::new();
    let mut dirs = vec![root.clone()];
    while let Some(dir) = dirs.pop() {
        println!("cargo:rerun-if-changed={}", dir.display());
        for entry in std::fs::read_dir(&dir).expect("assets/images") {
            let path = entry.unwrap().path();
            if path.is_dir() {
                dirs.push(path);
            } else if path.extension().is_some_and(|e| e == "webp" || e == "svg") {
                files.push(path);
            }
        }
    }
    files.sort();
    let mut code = format!("pub static FILES: [(&str, &[u8]); {}] = [\n", files.len());
    for path in &files {
        let rel = path
            .strip_prefix(&root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        code.push_str(&format!(
            "    ({rel:?}, include_bytes!({:?})),\n",
            path.to_string_lossy()
        ));
    }
    code.push_str("];\n");
    let out = PathBuf::from(env::var("OUT_DIR").unwrap()).join("image_files.rs");
    std::fs::write(out, code).expect("image_files.rs");
}

fn build_macos_editor_bridge() {
    let preview_source = PathBuf::from("native/macos/AeroShootPreview.swift");
    let media_source = PathBuf::from("native/macos/AeroShootMedia.swift");
    let playback_source = PathBuf::from("native/macos/AeroShootPlayback.swift");
    let export_source = PathBuf::from("native/macos/AeroShootExport.swift");

    println!("cargo:rerun-if-changed={}", preview_source.display());
    println!("cargo:rerun-if-changed={}", media_source.display());
    println!("cargo:rerun-if-changed={}", playback_source.display());
    println!("cargo:rerun-if-changed={}", export_source.display());

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is set by Cargo"));
    let library = out_dir.join("libaeroedits_macos.a");
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").expect("Cargo target architecture");
    let swift_target = format!("{target_arch}-apple-macosx13.0");
    let module_cache = out_dir.join("swift-module-cache");
    std::fs::create_dir_all(&module_cache).expect("create Swift module cache");

    let status = Command::new("xcrun")
        .env("CLANG_MODULE_CACHE_PATH", &module_cache)
        .args([
            "swiftc",
            "-target",
            &swift_target,
            "-swift-version",
            "5",
            "-parse-as-library",
            "-O",
            "-emit-library",
            "-static",
            "-module-name",
            "AeroShootEditor",
        ])
        .arg(&preview_source)
        .arg(&media_source)
        .arg(&export_source)
        .arg(&playback_source)
        .arg("-o")
        .arg(&library)
        .status()
        .expect("failed to invoke swiftc for the macOS editor bridge");
    assert!(
        status.success(),
        "swiftc failed to build the macOS editor bridge"
    );

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=aeroedits_macos");
    for framework in [
        "AVFoundation",
        "AppKit",
        "CoreAudio",
        "CoreFoundation",
        "CoreGraphics",
        "CoreImage",
        "CoreMedia",
        "CoreVideo",
        "Foundation",
        "QuartzCore",
        "VideoToolbox",
        "AudioToolbox",
    ] {
        println!("cargo:rustc-link-lib=framework={framework}");
    }
    println!("cargo:rustc-link-search=native=/usr/lib/swift");
    println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");
    println!("cargo:rustc-link-arg=-Wl,-rpath,/Library/Developer/CommandLineTools/usr/lib/swift-5.5/macosx");
}
