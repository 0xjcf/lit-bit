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

echo "✅ $ACTION_DESCRIPTION complete for $APP_NAME (workspace)." 