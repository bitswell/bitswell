# Plan: Create PR for LOOM Evaluation Plan v10

## Overview

This task commits the evaluation plan file `tests/loom-eval/plans/plan-v10.md` (already present in the worktree), pushes the `loom/plan-v10` branch to origin, and creates a GitHub pull request targeting `main`.

The plan file describes a **continuation agent test** -- an end-to-end evaluation of the resource-limit recovery recipe where a primary agent exhausts its token budget, enters BLOCKED state, and an orchestrator spawns a continuation agent to finish the work.

## Steps

### Step 1: Commit the plan file

- Stage `tests/loom-eval/plans/plan-v10.md`.
- Commit with a descriptive message and required LOOM trailers (`Agent-Id: plan-v10`, `Session-Id: 86a83d65-8905-439e-b409-79d2d7759cdd`).

### Step 2: Push the branch

- Push `loom/plan-v10` to `origin` with `-u` to set upstream tracking.

### Step 3: Create the pull request

- Use `gh pr create` targeting `main`.
- Title: "Add LOOM evaluation plan v10: continuation agent test"
- Body: Summary of what plan v10 covers (resource-limit recovery, BLOCKED state, continuation agents, dynamic agent creation).

### Step 4: Update STATUS.md to COMPLETED

- Set `status: COMPLETED` in STATUS.md.
- Write MEMORY.md with required sections.
- Commit both with LOOM trailers.

## Risks

- Push may fail if the remote branch already exists or if SSH access is misconfigured. Mitigation: check remote state before pushing.
- PR creation may fail if a PR already exists for this branch. Mitigation: check for existing PRs first.

## Scope Compliance

Only `tests/loom-eval/plans/plan-v10.md` is modified within the allowed paths. STATUS.md, PLAN.md, and MEMORY.md are protocol files in the worktree root.
