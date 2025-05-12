#!/bin/bash
# scripts/check-progress-log.sh

# Exit immediately if a command exits with a non-zero status.
set -e

# Files patterns that indicate significant work was likely done
# We check *.rs, Cargo.*, Spec.md, ROADMAP.md, prompts files (excluding PROGRESS.md itself), and rule files.
trigger_patterns='(\.(rs|md|mdc)$|Cargo\.toml|Cargo\.lock|^(Spec|ROADMAP)\.md$|^prompts/|^\.cursor/rules/)'

# Get list of staged files matching trigger patterns, excluding PROGRESS.md
trigger_files=$(git diff --cached --name-only --diff-filter=ACM | grep -E "$trigger_patterns" | grep -v 'prompts/project/PROGRESS\.md$' || true)

# Check if PROGRESS.md itself is staged
progress_staged=$(git diff --cached --name-only --diff-filter=ACM | grep -E '^prompts/project/PROGRESS\.md$' || true)

# If trigger files were staged, but PROGRESS.md was not...
if [[ -n "$trigger_files" && -z "$progress_staged" ]]; then
  echo "-----------------------------------------------------------------" >&2
  echo "COMMIT REJECTED: Found changes in spec/source/prompt files:" >&2
  echo "$trigger_files" | sed 's/^/  - /' >&2 # List files found
  echo "" >&2
  echo "But detected no corresponding changes staged in:" >&2
  echo "  prompts/project/PROGRESS.md" >&2
  echo "" >&2
  echo "Please stage an update to PROGRESS.md describing the work done." >&2
  echo "(See .cursor/rules/progress_log.mdc for guidelines)." >&2
  echo "-----------------------------------------------------------------" >&2
  exit 1 # Reject the commit
fi

# If we get here, the check passed
exit 0 