//! Heap/Unsafe Safety Check Script
//! Parses `geiger_report.json` and fails if any unsafe code is used in lit-bit-core itself.

use std::fs::File;
use std::io::BufReader;
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
    id: String,
    name: String,
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

/// Checks the `geiger_report.json` file for unsafe code usage in the `lit-bit-core` package.
///
/// Reads and parses the Geiger safety analysis report, locates the `lit-bit-core` package, and sums all unsafe code usages across functions, expressions, implementations, traits, and methods. If any unsafe code is detected, prints detailed counts and exits with a non-zero status. If the package is not found, prints an error and exits with failure. Otherwise, prints a success message indicating no unsafe code.
///
/// # Panics
///
/// Panics if the `geiger_report.json` file cannot be opened or parsed.
///
/// # Examples
///
/// ```no_run
/// // Run as a standalone binary to check for unsafe code in lit-bit-core.
/// // Exits with code 1 if unsafe code is found or the package is missing.
/// main();
/// ```
fn main() {
    let file = File::open("geiger_report.json").expect("Failed to open geiger_report.json");
    let reader = BufReader::new(file);
    let report: GeigerReport =
        serde_json::from_reader(reader).expect("Failed to parse geiger_report.json");

    // Look for lit-bit-core specifically
    let mut total_unsafe = 0u64;
    let mut found_lit_bit_core = false;

    for package_entry in &report.packages {
        let package_name = &package_entry.package.name;
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
