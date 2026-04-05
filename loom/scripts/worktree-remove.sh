#!/usr/bin/env bash
# loom/scripts/worktree-remove.sh
#
# Hook: WorktreeRemove (non-blockable)
#
# Logs cleanup and verifies the branch reached a terminal Task-Status
# (COMPLETED or FAILED) before the worktree was removed.
#
# Non-loom branches are a graceful no-op (exit 0).
#
# stdin: JSON with at least { "branch": "...", "path": "...", "cwd": "...", "session_id": "..." }

set -euo pipefail

INPUT=$(cat)

BRANCH=$(printf '%s' "$INPUT" | jq -r '.branch // .worktree_branch // ""' 2>/dev/null || true)
WORKTREE_PATH=$(printf '%s' "$INPUT" | jq -r '.path // ""' 2>/dev/null || true)
TIMESTAMP=$(date -u +%Y-%m-%dT%H:%M:%SZ)

# Not a loom/* branch — graceful no-op
if [[ -z "$BRANCH" ]] || [[ "$BRANCH" != loom/* ]]; then
  exit 0
fi

printf 'LOOM: WorktreeRemove: branch=%s path=%s time=%s\n' "$BRANCH" "$WORKTREE_PATH" "$TIMESTAMP"

# Check terminal Task-Status on the branch
if git rev-parse --verify "$BRANCH" >/dev/null 2>&1; then
  LAST_STATUS=$(git log -1 \
    --format='%(trailers:key=Task-Status,valueonly)' \
    "$BRANCH" 2>/dev/null | head -1 | tr -d '[:space:]')

  case "$LAST_STATUS" in
    COMPLETED|FAILED)
      printf 'LOOM: Terminal status confirmed: Task-Status: %s\n' "$LAST_STATUS"
      ;;
    "")
      printf 'LOOM: WARN: No Task-Status trailer found on branch %s\n' "$BRANCH" >&2
      printf 'LOOM: WARN: Worktree removed without reaching terminal state\n' >&2
      ;;
    *)
      printf 'LOOM: WARN: Non-terminal Task-Status "%s" on branch %s\n' "$LAST_STATUS" "$BRANCH" >&2
      printf 'LOOM: WARN: Expected COMPLETED or FAILED before worktree removal\n' >&2
      ;;
  esac
else
  printf 'LOOM: WARN: Branch %s not found in git refs\n' "$BRANCH" >&2
fi

exit 0
