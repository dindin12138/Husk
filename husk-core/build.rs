use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_path = PathBuf::from(&out_dir);
    let ghostty_src_dir = out_path.join("ghostty");

    // Clone Ghostty source code if it doesn't exist
    if !ghostty_src_dir.exists() {
        println!("cargo:warning=Fetching libghostty source...");

        let status = Command::new("git")
            .args([
                "clone",
                "https://github.com/ghostty-org/ghostty.git",
                ghostty_src_dir.to_str().unwrap(),
            ])
            .status()
            .expect("Git clone failed");
        assert!(status.success(), "Failed to fetch Ghostty");

        let status = Command::new("git")
            .current_dir(&ghostty_src_dir)
            .args(["checkout", "b839561e5db36589d6a999044c76bfe785a013d7"])
            .status()
            .expect("Git checkout failed");
        assert!(status.success(), "Failed to checkout commit");
    }

    // Compile the project using Zig's default install step
    println!("cargo:warning=Compiling libghostty with Zig (this may take a few minutes)...");
    let status = Command::new("zig")
        .current_dir(&ghostty_src_dir)
        .args(["build", "-Doptimize=ReleaseSafe", "-Dapp-runtime=none"])
        .status()
        .expect("Zig execution failed");

    assert!(status.success(), "Failed to compile libghostty");

    // Locate build artifacts
    let include_dir = ghostty_src_dir.join("zig-out/include");
    let lib_dir = ghostty_src_dir.join("zig-out/lib");

    // Configure linker search path and library
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=ghostty-vt");

    // Generate Rust FFI bindings
    println!("cargo:warning=Generating Rust FFI bindings...");
    let header_path = include_dir.join("ghostty/vt.h");

    if !header_path.exists() {
        panic!(
            "Critical error: vt.h not found at {}",
            header_path.display()
        );
    }

    println!("cargo:rerun-if-changed={}", header_path.display());

    let bindings = bindgen::Builder::default()
        .header(header_path.to_str().unwrap())
        .clang_arg(format!("-I{}", include_dir.display()))
        .derive_debug(true)
        .derive_default(true)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Bindgen failed to parse vt.h");

    bindings
        .write_to_file(out_path.join("ghostty_bindings.rs"))
        .expect("Failed to write bindings file");
}
