# Plan: Create PR for LOOM Evaluation Plan v9

## Overview

This agent's job is to commit the evaluation plan file `tests/loom-eval/plans/plan-v9.md`, push the branch `loom/plan-v9` to origin, and create a GitHub pull request targeting `main`.

The plan file (plan-v9.md) describes a "Merge Conflict Gauntlet" evaluation scenario that stress-tests every merge conflict recovery path in the LOOM protocol by deliberately assigning agents overlapping file scopes.

## Steps

1. **Stage the plan file** -- `git add tests/loom-eval/plans/plan-v9.md`
2. **Commit with LOOM trailers** -- Commit message includes `Agent-Id: plan-v9` and `Session-Id: fdc7fdbc-9332-4d2f-bc53-2de89857dd9a` trailers.
3. **Stage and commit STATUS.md, PLAN.md, and MEMORY.md** -- These LOOM lifecycle files are committed alongside or after the plan file.
4. **Push the branch** -- `git push -u origin loom/plan-v9`
5. **Create the PR** -- Use `gh pr create` targeting `main` with a descriptive title and body summarizing the plan-v9 evaluation scenario.
6. **Update STATUS.md to COMPLETED** -- Final commit marking the task as done.

## Risks

- Push may fail if origin is unreachable or branch already exists remotely. Mitigation: check remote state before pushing.
- PR creation may fail if a PR already exists for this branch. Mitigation: check for existing PRs first.

## Scope

Only `tests/loom-eval/plans/plan-v9.md` is in the allowed paths for the plan content. STATUS.md, PLAN.md, and MEMORY.md are LOOM protocol files written at the worktree root.
