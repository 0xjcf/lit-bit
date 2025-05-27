#!/bin/bash
# scripts/check-progress-log.sh

# Exit immediately if a command exits with a non-zero status.
set -e

# Files patterns that indicate significant work was likely done
# We check *.rs, Cargo.*, Spec.md, ROADMAP.md, prompts files, and rule files.
# docs/PROGRESS/ files are handled separately.
trigger_patterns='(\.(rs|md|mdc)$|Cargo\.toml|Cargo\.lock|^(Spec|ROADMAP)\.md$|^prompts/|^\.cursor/rules/)'

# Get list of staged files matching trigger patterns, excluding progress files
trigger_files=$(git diff --cached --name-only --diff-filter=ACM | grep -E "$trigger_patterns" | grep -v '^docs/PROGRESS/' || true)

# Check if any progress files are staged
progress_staged=$(git diff --cached --name-only --diff-filter=ACM | grep -E '^docs/PROGRESS/.*\.md$' || true)

# If trigger files were staged, but no progress files were staged...
if [[ -n "$trigger_files" && -z "$progress_staged" ]]; then
  echo "-----------------------------------------------------------------" >&2
  echo "COMMIT REJECTED: Found changes in spec/source/prompt files:" >&2
  echo "$trigger_files" | sed 's/^/  - /' >&2
  echo "" >&2
  echo "But detected no corresponding progress file updates in docs/PROGRESS/" >&2
  echo "" >&2
  
  # Get current date and suggest filename
  current_date=$(date +%Y-%m-%d)
  suggested_file="docs/PROGRESS/${current_date}.md"
  
  echo "Suggestions:" >&2
  echo "  1. Create/update: $suggested_file" >&2
  echo "  2. Or update an existing recent progress file" >&2
  echo "" >&2
  echo "Progress files should contain:" >&2
  echo "  - Session summary with author, phase, branch" >&2
  echo "  - Work completed details" >&2
  echo "  - Git commit references" >&2
  echo "  - Testing status" >&2
  echo "" >&2
  echo "See docs/PROGRESS/README.md for format guidelines." >&2
  echo "-----------------------------------------------------------------" >&2
  exit 1
fi

# If progress files are staged, validate they have minimal content
if [[ -n "$progress_staged" ]]; then
  echo "Validating staged progress files..." >&2
  
  for file in $progress_staged; do
    if [[ -f "$file" ]]; then
      # Check for required sections
      if ! grep -q "## Session Summary" "$file"; then
        echo "ERROR: $file missing '## Session Summary' section" >&2
        exit 1
      fi
      
      if ! grep -q "## Work Completed" "$file"; then
        echo "ERROR: $file missing '## Work Completed' section" >&2
        exit 1
      fi
      
      # Check minimum content length (should be more than just headers)
      line_count=$(wc -l < "$file")
      if [[ $line_count -lt 15 ]]; then
        echo "ERROR: $file appears too short (less than 15 lines)" >&2
        echo "Progress files should contain meaningful work descriptions" >&2
        exit 1
      fi
      
      echo "✓ $file validated" >&2
    fi
  done
fi

# If we get here, the check passed
echo "Progress log check passed ✓" >&2
exit 0 