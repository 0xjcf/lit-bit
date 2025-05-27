//! Heap/Unsafe Safety Check Script
//! Parses `geiger_report.json` and fails if any unsafe code is used in lit-bit-core itself.

use std::fs::File;
use std::io::{BufReader, Read};
use std::process;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct GeigerReport {
    packages: Vec<GeigerPackageEntry>,
}

#[derive(Debug, Deserialize)]
struct GeigerPackageEntry {
    package: GeigerPackageInfo,
    unsafety: UnsafetyInfo,
}

#[derive(Debug, Deserialize)]
struct GeigerPackageInfo {
    #[allow(dead_code)]
    id: GeigerPackageId,
}

#[derive(Debug, Deserialize)]
struct GeigerPackageId {
    name: String,
    #[allow(dead_code)]
    version: String,
    #[allow(dead_code)]
    source: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct UnsafetyInfo {
    used: UnsafeStats,
}

#[derive(Debug, Deserialize)]
struct UnsafeStats {
    #[serde(default)]
    functions: UnsafeCount,
    #[serde(default)]
    exprs: UnsafeCount,
    #[serde(rename = "impls", default)]
    item_impls: UnsafeCount,
    #[serde(rename = "traits", default)]
    item_traits: UnsafeCount,
    #[serde(default)]
    methods: UnsafeCount,
}

#[derive(Debug, Deserialize, Default)]
struct UnsafeCount {
    #[serde(rename = "unsafe_")]
    unsafe_count: u64,
}

fn main() {
    let file = File::open("geiger_report.json").expect("Failed to open geiger_report.json");
    let reader = BufReader::new(file);

    // Check for debug environment variable
    let debug_enabled = std::env::var("DEBUG_GEIGER").is_ok();

    let report: GeigerReport = if debug_enabled {
        // Read the entire file content for debugging
        let file_debug =
            File::open("geiger_report.json").expect("Failed to open geiger_report.json for debug");
        let mut reader_debug = BufReader::new(file_debug);
        let mut content = String::new();
        reader_debug
            .read_to_string(&mut content)
            .expect("Failed to read geiger_report.json content");

        eprintln!("🐛 DEBUG_GEIGER: Raw JSON content (first 500 chars):");
        eprintln!("{}", &content.chars().take(500).collect::<String>());
        if content.len() > 500 {
            eprintln!("... (truncated, total length: {} chars)", content.len());
        }

        // Try to parse as generic JSON first to see structure
        match serde_json::from_str::<serde_json::Value>(&content) {
            Ok(json_value) => {
                eprintln!("🐛 DEBUG_GEIGER: Successfully parsed as generic JSON");
                eprintln!("🐛 DEBUG_GEIGER: JSON structure: {json_value:#}");
            }
            Err(e) => {
                eprintln!("🐛 DEBUG_GEIGER: Failed to parse even as generic JSON: {e}");
            }
        }

        // Now try to parse as our specific structure
        match serde_json::from_str::<GeigerReport>(&content) {
            Ok(report) => {
                eprintln!("🐛 DEBUG_GEIGER: Successfully parsed as GeigerReport");
                report
            }
            Err(e) => {
                eprintln!("🐛 DEBUG_GEIGER: Failed to parse as GeigerReport: {e}");
                eprintln!(
                    "🐛 DEBUG_GEIGER: This indicates a mismatch between expected and actual JSON structure"
                );
                panic!("Failed to parse geiger_report.json: {e}");
            }
        }
    } else {
        serde_json::from_reader(reader).expect("Failed to parse geiger_report.json")
    };

    // Look for lit-bit-core specifically
    let mut total_unsafe = 0u64;
    let mut found_lit_bit_core = false;

    for package_entry in &report.packages {
        let package_name = &package_entry.package.id.name;
        if package_name == "lit-bit-core" {
            found_lit_bit_core = true;
            let unsafety = &package_entry.unsafety.used;

            total_unsafe += unsafety.functions.unsafe_count;
            total_unsafe += unsafety.exprs.unsafe_count;
            total_unsafe += unsafety.item_impls.unsafe_count;
            total_unsafe += unsafety.item_traits.unsafe_count;
            total_unsafe += unsafety.methods.unsafe_count;

            if total_unsafe > 0 {
                eprintln!("❌ lit-bit-core uses unsafe code! ({total_unsafe} total)");
                eprintln!("  Functions: {}", unsafety.functions.unsafe_count);
                eprintln!("  Expressions: {}", unsafety.exprs.unsafe_count);
                eprintln!("  Impls: {}", unsafety.item_impls.unsafe_count);
                eprintln!("  Traits: {}", unsafety.item_traits.unsafe_count);
                eprintln!("  Methods: {}", unsafety.methods.unsafe_count);
                process::exit(1);
            }
            break;
        }
    }

    if !found_lit_bit_core {
        eprintln!("❌ lit-bit-core not found in geiger report!");
        process::exit(1);
    }

    println!("✅ lit-bit-core contains no unsafe code (used: {total_unsafe})");
}
