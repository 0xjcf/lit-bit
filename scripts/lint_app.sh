#!/usr/bin/env bash
set -e

APP_NAME=$1
FIX_MODE=$2 # Second argument can be 'fix'

if [[ -z "$APP_NAME" ]]; then
    echo "❌ App name not provided to lint script."
    echo "Usage: ./scripts/lint_app.sh <app_name_or_workspace> [fix]"
    exit 1
fi

ACTION_DESCRIPTION="Linting"
if [[ "$FIX_MODE" == "fix" ]]; then
    ACTION_DESCRIPTION="Linting & fixing"
fi

echo "💅 $ACTION_DESCRIPTION $APP_NAME (operations are currently workspace-wide)..."

if [[ "$FIX_MODE" == "fix" ]]; then
    # Format all crates in the workspace
    cargo fmt --all
    # Lint and fix all crates in the workspace
    cargo clippy --workspace --all-targets --all-features --fix --allow-dirty --allow-staged -- -W clippy::pedantic -D warnings
else
    # Check formatting for all crates
    if ! cargo fmt --all --check; then
        echo "❌ Formatting issues found in $APP_NAME (workspace) by 'cargo fmt --all --check'."
        exit 1
    fi
    # Lint all crates, -D warnings will cause non-zero exit on issues
    cargo clippy --workspace --all-targets --all-features -- -W clippy::pedantic -D warnings
fi

# Check if nightly toolchain is available and run nightly clippy
echo ""
echo "🌙 Checking nightly clippy for future compatibility..."
if rustup toolchain list | grep -q "nightly"; then
    if cargo +nightly clippy --workspace --all-targets --all-features -- -D warnings; then
        echo "✅ Nightly clippy passed - CI should be happy!"
    else
        echo "⚠️  Nightly clippy found issues. These will cause CI failures."
        echo "   Consider fixing with 'cargo +nightly clippy --fix' or adding #[allow] attributes"
        # Don't exit 1 here - this is informational for now
    fi
else
    echo "ℹ️  Nightly toolchain not installed - skipping nightly clippy check"
    echo "   Install with: rustup toolchain install nightly"
fi

# Also run CI-exact checks to catch issues that only appear without --all-features
echo ""
echo "🤖 Running CI-exact checks (without --all-features)..."
if cargo clippy --all-targets --workspace -- -D warnings; then
    echo "✅ CI-exact clippy passed!"
else
    echo "❌ CI-exact clippy failed - this will cause CI failures!"
    echo "   This is the exact command CI runs: cargo clippy --all-targets --workspace -- -D warnings"
    exit 1
fi

echo "✅ $ACTION_DESCRIPTION complete for $APP_NAME (workspace)." 