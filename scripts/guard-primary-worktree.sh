#!/usr/bin/env bash
# guard-primary-worktree.sh — enforce: primary worktree HEAD is always `main`.
#
# The primary working tree at the repo root must never be checked out to a
# feature branch or commit directly. All feature work, orchestrator assigns,
# planner task decompositions, tool-requests, etc. happen in worktrees under
# .loom/... and land on main via PR.
#
# This script is invoked by scripts/hooks/pre-commit (hard block) and
# scripts/hooks/post-checkout (warning only). In any .loom/... worktree
# it is a no-op.
#
# Exit codes:
#   0 — allowed
#   1 — blocked (primary worktree on non-main branch)

set -euo pipefail

MODE="${1:-block}"   # "block" or "warn"

repo_top=$(git rev-parse --show-toplevel 2>/dev/null) || exit 0
common_dir=$(git rev-parse --git-common-dir 2>/dev/null) || exit 0
common_dir_abs=$(cd "$common_dir" && pwd)
primary_top=$(dirname "$common_dir_abs")

if [[ "$repo_top" != "$primary_top" ]]; then
  # We are inside a linked worktree (e.g. .loom/agents/moss/worktrees/...) — no-op.
  exit 0
fi

current_branch=$(git symbolic-ref --short HEAD 2>/dev/null || echo "")

if [[ "$current_branch" == "main" ]]; then
  exit 0
fi

msg="primary worktree ($repo_top) must be on 'main', got '${current_branch:-detached HEAD}'.
Work in a linked worktree instead:
    git worktree add .loom/orchestrator/<slug> -b loom/orchestrator-<slug> origin/main
Then open a PR against main from that worktree."

if [[ "$MODE" == "warn" ]]; then
  echo "guard-primary-worktree: warning — $msg" >&2
  exit 0
fi

echo "guard-primary-worktree: BLOCKED — $msg" >&2
exit 1
