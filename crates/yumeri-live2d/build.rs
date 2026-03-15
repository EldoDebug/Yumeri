use std::{env, path::PathBuf};

fn main() {
    let target = env::var("TARGET").expect("TARGET is set by Cargo");
    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let bundled_core_dir = manifest_dir.join("vendor/live2d/cubism-core/5-r.4.1");
    let core_dir = env::var("CUBISM_CORE_DIR")
        .map(PathBuf::from)
        .unwrap_or(bundled_core_dir);

    println!("cargo:rerun-if-env-changed=CUBISM_CORE_DIR");
    let header = core_dir.join("include/Live2DCubismCore.h");
    println!("cargo:rerun-if-changed={}", header.display());
    if !header.is_file() {
        panic!(
            "Cubism Core header not found: {} (set CUBISM_CORE_DIR to a directory containing include/ and lib/ or dll/)",
            header.display()
        );
    }

    let is_static = env::var_os("CARGO_FEATURE_STATIC_").is_some();
    let is_dynamic = env::var_os("CARGO_FEATURE_DYNAMIC").is_some();
    if is_static == is_dynamic {
        panic!("Exactly one of features `dynamic` or `static_` must be enabled.");
    }

    if target.contains("windows") {
        link_windows(&core_dir, &target, &profile, is_static);
        return;
    }

    if target.contains("apple-darwin") {
        link_macos(&core_dir, &target, is_static);
        return;
    }

    if target.contains("linux") {
        link_linux(&core_dir, &target, is_static);
        return;
    }

    panic!("Unsupported target: {target}");
}

fn link_windows(core_dir: &std::path::Path, target: &str, profile: &str, is_static: bool) {
    let arch = if target.contains("x86_64") {
        "x86_64"
    } else if target.contains("i686") || target.contains("x86") {
        "x86"
    } else {
        panic!("Unsupported Windows arch for Cubism Core: {target}");
    };

    if is_static {
        let base = core_dir.join(format!("lib/windows/{arch}"));
        if !base.is_dir() {
            panic!(
                "Cubism Core static libraries not found under: {}",
                base.display()
            );
        }
        let msvc_ver = pick_latest_numeric_subdir(&base).unwrap_or_else(|| "143".to_string());

        let use_mt = env::var_os("CARGO_FEATURE_WINDOWS_MSVC_MT").is_some();

        let crt = if use_mt { "MT" } else { "MD" };
        let debug_suffix = if profile == "debug" { "d" } else { "" };
        let lib_name = format!("Live2DCubismCore_{crt}{debug_suffix}");

        let search = base.join(msvc_ver);
        println!("cargo:rustc-link-search=native={}", search.display());
        println!("cargo:rustc-link-lib=static={lib_name}");
        return;
    }

    let search = core_dir.join(format!("dll/windows/{arch}"));
    let dll = search.join("Live2DCubismCore.dll");
    let import_lib = search.join("Live2DCubismCore.lib");
    println!("cargo:rerun-if-changed={}", dll.display());
    println!("cargo:rerun-if-changed={}", import_lib.display());
    if !dll.is_file() || !import_lib.is_file() {
        panic!(
            "Cubism Core dynamic libraries not found: {} / {}",
            dll.display(),
            import_lib.display()
        );
    }
    println!("cargo:rustc-link-search=native={}", search.display());
    println!("cargo:rustc-link-lib=dylib=Live2DCubismCore");

    stage_windows_dll(&dll, profile);
}

fn pick_latest_numeric_subdir(dir: &std::path::Path) -> Option<String> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut best: Option<(u32, String)> = None;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let Ok(num) = name.parse::<u32>() else {
            continue;
        };
        if entry.file_type().ok().is_some_and(|t| t.is_dir()) {
            match best.as_ref() {
                Some((best_num, _)) if *best_num >= num => {}
                _ => best = Some((num, name)),
            }
        }
    }
    best.map(|(_, name)| name)
}

fn link_macos(core_dir: &std::path::Path, target: &str, is_static: bool) {
    let arch = if target.contains("aarch64") {
        "arm64"
    } else if target.contains("x86_64") {
        "x86_64"
    } else {
        panic!("Unsupported macOS arch for Cubism Core: {target}");
    };

    if is_static {
        let search = core_dir.join(format!("lib/macos/{arch}"));
        let lib = search.join("libLive2DCubismCore.a");
        println!("cargo:rerun-if-changed={}", lib.display());
        if !lib.is_file() {
            panic!(
                "Cubism Core static library not found: {}",
                lib.display()
            );
        }
        println!("cargo:rustc-link-search=native={}", search.display());
        println!("cargo:rustc-link-lib=static=Live2DCubismCore");
        return;
    }

    let search = core_dir.join("dll/macos");
    let dylib = search.join("libLive2DCubismCore.dylib");
    println!("cargo:rerun-if-changed={}", dylib.display());
    if !dylib.is_file() {
        panic!(
            "Cubism Core dynamic library not found: {}",
            dylib.display()
        );
    }
    println!("cargo:rustc-link-search=native={}", search.display());
    println!("cargo:rustc-link-lib=dylib=Live2DCubismCore");
}

fn link_linux(core_dir: &std::path::Path, target: &str, is_static: bool) {
    if !target.contains("x86_64") {
        panic!("Unsupported Linux arch for Cubism Core (currently x86_64 only): {target}");
    }

    if is_static {
        let search = core_dir.join("lib/linux/x86_64");
        let lib = search.join("libLive2DCubismCore.a");
        println!("cargo:rerun-if-changed={}", lib.display());
        if !lib.is_file() {
            panic!(
                "Cubism Core static library not found: {}",
                lib.display()
            );
        }
        println!("cargo:rustc-link-search=native={}", search.display());
        println!("cargo:rustc-link-lib=static=Live2DCubismCore");
        return;
    }

    let search = core_dir.join("dll/linux/x86_64");
    let so = search.join("libLive2DCubismCore.so");
    println!("cargo:rerun-if-changed={}", so.display());
    if !so.is_file() {
        panic!(
            "Cubism Core dynamic library not found: {}",
            so.display()
        );
    }
    println!("cargo:rustc-link-search=native={}", search.display());
    println!("cargo:rustc-link-lib=dylib=Live2DCubismCore");
}

fn stage_windows_dll(dll: &std::path::Path, profile: &str) {
    let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") else {
        return;
    };
    let manifest_dir = PathBuf::from(manifest_dir);
    // Navigate to workspace root (crates/yumeri-live2d -> workspace root)
    let workspace_root = manifest_dir.join("../..");

    let target_dir = env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| workspace_root.join("target"));
    let dll_dst = target_dir.join(profile).join("Live2DCubismCore.dll");
    let _ = std::fs::create_dir_all(dll_dst.parent().unwrap());

    let copy = match (std::fs::metadata(dll), std::fs::metadata(&dll_dst)) {
        (Ok(src), Ok(dst)) => src.modified().ok() > dst.modified().ok(),
        (Ok(_), Err(_)) => true,
        _ => false,
    };
    if copy {
        let _ = std::fs::copy(dll, &dll_dst);
    }
}
