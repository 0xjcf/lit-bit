#!/bin/bash
set -euo pipefail

# Professional-grade linting script with feature matrix testing
# Never skips errors - fails fast on any issues

echo "🔍 Running comprehensive workspace linting..."

# Test core library with each valid feature combination
echo "📦 Testing lit-bit-core feature combinations..."

# Test library only (no examples) with no features
echo "  ✅ Testing no-default-features (lib only)..."
cargo clippy -p lit-bit-core --lib --no-default-features -- -D warnings

# Test library only with each async runtime
echo "  ✅ Testing async-tokio (lib only)..."
cargo clippy -p lit-bit-core --lib --features async-tokio -- -D warnings

echo "  ✅ Testing async-embassy (lib only)..."  
cargo clippy -p lit-bit-core --lib --features async-embassy -- -D warnings

# Test library with std features
echo "  ✅ Testing std features (lib only)..."
cargo clippy -p lit-bit-core --lib --features std -- -D warnings

# Test examples with their required features
echo "📦 Testing examples with required features..."

echo "  ✅ Testing std examples..."
cargo clippy -p lit-bit-core --example actor_backpressure --features std -- -D warnings
cargo clippy -p lit-bit-core --example external_events --features std -- -D warnings
cargo clippy -p lit-bit-core --example actor_statechart_integration --features std -- -D warnings

echo "  ✅ Testing async examples..."  
cargo clippy -p lit-bit-core --example embassy_actor_simple --features async-embassy,debug-log -- -D warnings
cargo clippy -p lit-bit-core --example async_actor_simple --features async-tokio -- -D warnings
cargo clippy -p lit-bit-core --example supervision_and_batching --features async-tokio -- -D warnings
cargo clippy -p lit-bit-core --example actor_calculator --features async-tokio -- -D warnings

echo "  ✅ Testing no_std examples..."
cargo clippy -p lit-bit-core --example media_player --no-default-features -- -D warnings

echo "  ✅ Testing no_std examples with panic-halt..."
cargo clippy -p lit-bit-core --example heap_crash --features panic-halt -- -D warnings
cargo clippy -p lit-bit-core --example actor_simple_usage --features panic-halt -- -D warnings
cargo clippy -p lit-bit-core --example traffic_light --features panic-halt -- -D warnings
cargo clippy -p lit-bit-core --example traffic_light_cortex_m --features panic-halt -- -D warnings

# Test other workspace members
echo "📦 Testing other workspace members..."

if [ -d "lit-bit-macro" ]; then
    echo "  ✅ Testing lit-bit-macro..."
    cargo clippy -p lit-bit-macro --all-targets -- -D warnings
fi

if [ -d "lit-bit-tests" ]; then
    echo "  ✅ Testing lit-bit-tests with async-tokio..."
    cargo clippy -p lit-bit-tests --all-targets --features async-tokio -- -D warnings
    
    # Note: lit-bit-tests is currently Tokio-specific, so we don't test Embassy features
    # In a real project, we'd have separate Embassy-specific tests
fi

if [ -d "lit-bit-bench" ]; then
    echo "  ✅ Testing lit-bit-bench..."
    cargo clippy -p lit-bit-bench --all-targets -- -D warnings
fi

# Test that mutually exclusive features properly fail
echo "📦 Testing mutually exclusive feature detection..."
echo "  ✅ Testing that async-tokio + async-embassy fails (expected)..."
if cargo check -p lit-bit-core --features async-tokio,async-embassy 2>/dev/null; then
    echo "❌ ERROR: Mutually exclusive features should fail but didn't!"
    exit 1
else
    echo "  ✅ Mutually exclusive features correctly rejected"
fi

# Test formatting
echo "📦 Testing code formatting..."
if ! cargo fmt --all --check; then
    echo "❌ Code formatting issues found. Run: cargo fmt --all"
    exit 1
fi

# Test nightly clippy if available
if rustup toolchain list | grep -q "nightly"; then
    echo "📦 Testing nightly clippy for future compatibility..."
    echo "  ✅ Testing core with nightly..."
    cargo +nightly clippy -p lit-bit-core --lib --features std -- -D warnings
else
    echo "ℹ️  Nightly toolchain not available, skipping nightly tests"
fi

echo ""
echo "✅ All linting checks passed!"
echo "✨ Code quality is excellent - no warnings or errors found" 