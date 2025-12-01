use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // Always use RP2350 linker script
    let script_name = "rp2350.x";
    fs::copy(script_name, out_dir.join("memory.x")).expect("Failed to copy rp2350.x to memory.x");
    
    println!("cargo:rustc-link-search={}", out_dir.display());
    println!("cargo:rerun-if-changed={}", script_name);

    // Handle defmt.x if it exists
    if PathBuf::from("defmt.x").exists() {
        fs::copy("defmt.x", out_dir.join("defmt.x")).expect("Failed to copy defmt.x");
        println!("cargo:rustc-link-search={}", out_dir.display());
        println!("cargo:rerun-if-changed=defmt.x");
    }
    
    println!("cargo:rerun-if-changed=build.rs");
}
