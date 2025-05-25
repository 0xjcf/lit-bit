//! Heap/Unsafe Safety Check Script
//! Parses geiger_report.json and fails if any unsafe code is used in lit-bit-core itself.

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
    id: PackageId,
}

#[derive(Debug, Deserialize)]
struct PackageId {
    name: String,
}

#[derive(Debug, Deserialize)]
struct UnsafetyInfo {
    used: UnsafeStats,
}

#[derive(Debug, Deserialize)]
struct UnsafeStats {
    functions: UnsafeCount,
    exprs: UnsafeCount,
    item_impls: UnsafeCount,
    item_traits: UnsafeCount,
    methods: UnsafeCount,
}

#[derive(Debug, Deserialize)]
struct UnsafeCount {
    #[serde(rename = "unsafe_")]
    unsafe_count: u64,
}

fn main() {
    let file = File::open("geiger_report.json").expect("Failed to open geiger_report.json");
    let reader = BufReader::new(file);
    let report: GeigerReport =
        serde_json::from_reader(reader).expect("Failed to parse geiger_report.json");

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
