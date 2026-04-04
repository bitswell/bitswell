# Plan: Create PR for LOOM Evaluation Plan v0

## Overview

Commit the evaluation plan file `tests/loom-eval/plans/plan-v0.md` to the `loom/plan-v0` branch, push to origin, and create a GitHub pull request targeting `main`.

## Steps

1. **Stage the plan file** -- `git add tests/loom-eval/plans/plan-v0.md`
2. **Commit with LOOM trailers** -- Commit message includes `Agent-Id: plan-v0` and `Session-Id: 8bf2c847-2440-4256-a721-78eae95dbc95` trailers.
3. **Push the branch** -- `git push -u origin loom/plan-v0`
4. **Create the PR** -- Use `gh pr create` targeting `main` with a descriptive title and body summarizing the plan variant.
5. **Update STATUS.md** -- Set status to `COMPLETED`.
6. **Write MEMORY.md** -- Record what was done and the PR URL.
7. **Final commit** -- Commit STATUS.md and MEMORY.md with trailers.

## Files Modified

- `tests/loom-eval/plans/plan-v0.md` (already present, just needs committing)
- `STATUS.md` (lifecycle tracking)
- `PLAN.md` (this file)
- `MEMORY.md` (handoff record)

## Risks

- Push may fail if remote branch already exists -- handle by force-pushing or verifying first.
- PR creation may fail if one already exists -- check first with `gh pr list`.
