#!/usr/bin/env bash
# loom/scripts/worktree-create.sh
#
# Hook: WorktreeCreate (blockable — exit 2 aborts creation)
#
# Validates that loom/* worktrees follow the branch naming convention:
#   loom/<slug>  (regex: ^loom/[a-z0-9]+(-[a-z0-9]+)*$)
#
# Non-loom branches are a graceful no-op (exit 0).
#
# stdin: JSON with at least { "branch": "...", "cwd": "...", "session_id": "..." }

set -euo pipefail

LOOM_PATTERN='^loom/[a-z0-9]+(-[a-z0-9]+)*$'

INPUT=$(cat)

# Extract branch from hook payload
BRANCH=$(printf '%s' "$INPUT" | jq -r '.branch // .worktree_branch // ""' 2>/dev/null || true)

# No branch info — can't validate, pass through
if [[ -z "$BRANCH" ]]; then
  exit 0
fi

# Not a loom/* branch — graceful no-op
if [[ "$BRANCH" != loom/* ]]; then
  exit 0
fi

# Validate against the LOOM branch naming convention
if ! printf '%s' "$BRANCH" | grep -qE "$LOOM_PATTERN"; then
  printf 'LOOM: Invalid branch name: %s\n' "$BRANCH" >&2
  printf 'LOOM: Branch must match: loom/<slug>  (lowercase, hyphens only)\n' >&2
  printf 'LOOM: Pattern: %s\n' "$LOOM_PATTERN" >&2
  printf 'LOOM: Examples: loom/pr-6, loom/ratchet-fix, loom/migrate-auth\n' >&2
  exit 2  # Block worktree creation
fi

printf 'LOOM: WorktreeCreate validated: %s\n' "$BRANCH"
exit 0
