#!/bin/bash
set -euo pipefail

# bitswell startup — activate the primary-worktree pre-commit guard and
# show the current task-branch backlog.
#
# Tasks live on `task/<project-slug>/<task-slug>` branches (empty-commit
# seed). Use `./startup.sh status` (the default) to see them; pick one
# up by branching a writer worktree off it via Shuttle-mode (see
# .claude/agents/shuttle.md).

REPO_DIR="$(cd "$(dirname "$0")" && pwd)"

# Activate versioned git hooks (primary-worktree guard).
git -C "$REPO_DIR" config core.hooksPath scripts/hooks

echo "Task branches:"
git -C "$REPO_DIR" for-each-ref --format='  %(refname:short)' refs/heads/task/ 2>/dev/null \
  || echo "  (none)"
