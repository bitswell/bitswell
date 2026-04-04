# Plan: Create PR for LOOM Evaluation Plan v1

## Overview

Commit the evaluation plan file `tests/loom-eval/plans/plan-v1.md` (an adversarial stress test for the LOOM orchestrator), push the branch `loom/plan-v1` to origin, and create a PR targeting `main`.

## Steps

1. **Stage and commit the plan file** -- `tests/loom-eval/plans/plan-v1.md` is already present in the worktree as an untracked file. Stage it and commit with a descriptive message and the required LOOM trailers (`Agent-Id: plan-v1`, `Session-Id: 0a00b1c7-6923-4739-940e-78b1328d3327`).

2. **Push the branch** -- Push `loom/plan-v1` to `origin` with the `-u` flag to set up tracking.

3. **Create the PR** -- Use `gh pr create` targeting `main` with a title describing the plan variant and a body summarizing the adversarial stress test content (7 violation categories, 7+ agents, scope/trailer/status/cycle/cross-worktree/state-transition/workspace-write tests).

4. **Update STATUS.md** -- Set status to `COMPLETED` once the PR is created.

5. **Write MEMORY.md** -- Record the PR URL and any relevant notes.

## Scope

- Only `tests/loom-eval/plans/plan-v1.md` is committed as content.
- `STATUS.md`, `PLAN.md`, and `MEMORY.md` are LOOM protocol files in the worktree root.

## Risks

- Push or PR creation could fail if there are network or permission issues. If so, retry once and report failure in STATUS.md.
