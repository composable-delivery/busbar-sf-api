//! Integration tests for WASM backend.
//!
//! These tests verify that the sf-client WASM backend can be compiled
//! and that the plugin has the expected structure.

use std::process::Command;
use std::path::PathBuf;

/// Test that the WASM plugin compiles successfully
#[test]
#[ignore] // Only run when explicitly requested
fn test_wasm_plugin_compiles() {
    let plugin_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("wasm-plugin");
    
    println!("Building WASM plugin at: {:?}", plugin_dir);
    
    // Check if wasm32-unknown-unknown target is installed
    let target_check = Command::new("rustup")
        .args(&["target", "list", "--installed"])
        .output()
        .expect("Failed to check installed targets");
    
    let targets = String::from_utf8_lossy(&target_check.stdout);
    if !targets.contains("wasm32-unknown-unknown") {
        eprintln!("wasm32-unknown-unknown target not installed.");
        eprintln!("Install with: rustup target add wasm32-unknown-unknown");
        panic!("Required WASM target not available");
    }
    
    // Build the WASM plugin
    let output = Command::new("cargo")
        .args(&[
            "build",
            "--target", "wasm32-unknown-unknown",
            "--release",
        ])
        .current_dir(&plugin_dir)
        .output()
        .expect("Failed to execute cargo build");
    
    if !output.status.success() {
        eprintln!("STDOUT:\n{}", String::from_utf8_lossy(&output.stdout));
        eprintln!("STDERR:\n{}", String::from_utf8_lossy(&output.stderr));
        panic!("WASM plugin compilation failed");
    }
    
    // Verify the WASM file exists
    let wasm_file = plugin_dir
        .join("target")
        .join("wasm32-unknown-unknown")
        .join("release")
        .join("sf_client_wasm_test_plugin.wasm");
    
    assert!(wasm_file.exists(), "WASM plugin file not found at {:?}", wasm_file);
    
    // Check that the file is not empty
    let metadata = std::fs::metadata(&wasm_file).expect("Failed to read WASM file metadata");
    assert!(metadata.len() > 0, "WASM plugin file is empty");
    
    println!("✓ WASM plugin compiled successfully: {:?}", wasm_file);
    println!("  Size: {} bytes", metadata.len());
}

/// Test that we can verify the WASM module structure using wasm-tools if available
#[test]
#[ignore] // Only run when explicitly requested
fn test_wasm_plugin_structure() {
    // First compile the plugin
    test_wasm_plugin_compiles();
    
    let plugin_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("wasm-plugin");
    
    let wasm_file = plugin_dir
        .join("target")
        .join("wasm32-unknown-unknown")
        .join("release")
        .join("sf_client_wasm_test_plugin.wasm");
    
    // Try to validate with wasm-tools if available
    let validation = Command::new("wasm-tools")
        .args(&["validate", wasm_file.to_str().unwrap()])
        .output();
    
    match validation {
        Ok(output) => {
            if output.status.success() {
                println!("✓ WASM module structure validated");
            } else {
                eprintln!("WASM validation output: {}", String::from_utf8_lossy(&output.stderr));
            }
        }
        Err(_) => {
            println!("⚠ wasm-tools not available, skipping structure validation");
            println!("  Install with: cargo install wasm-tools");
        }
    }
}

/// Document how to run the WASM tests manually
#[test]
fn test_wasm_manual_test_instructions() {
    println!("\n=== WASM Integration Test Instructions ===\n");
    println!("To manually test the WASM plugin:\n");
    println!("1. Build the plugin:");
    println!("   cd crates/sf-client/tests/wasm-plugin");
    println!("   cargo build --target wasm32-unknown-unknown --release\n");
    println!("2. Test with Extism CLI (if installed):");
    println!("   extism call target/wasm32-unknown-unknown/release/sf_client_wasm_test_plugin.wasm test_client_config --input '{{}}'");
    println!("   extism call target/wasm32-unknown-unknown/release/sf_client_wasm_test_plugin.wasm test_retry_rejected --input '{{}}'\n");
    println!("3. To run the compilation test:");
    println!("   cargo test --package busbar-sf-client --test test_wasm_integration --no-default-features --features wasm -- --ignored\n");
}
