# Plan: Create PR for LOOM Evaluation Plan v5

## Summary

This task commits the evaluation plan file `tests/loom-eval/plans/plan-v5.md` (Error Recovery Marathon) to branch `loom/plan-v5`, pushes the branch to origin, and creates a pull request targeting `main`.

## Steps

1. **Write PLAN.md and STATUS.md** (this step) -- Create the planning artifacts with `status: PLANNING`.
2. **Commit planning artifacts** -- Commit PLAN.md and STATUS.md with proper LOOM trailers (`Agent-Id: plan-v5`, `Session-Id: f6b392f8-cd1c-47a3-a9fc-c43a4944dfe9`).
3. **Stage and commit the plan file** -- `git add tests/loom-eval/plans/plan-v5.md` and commit with a descriptive message and LOOM trailers.
4. **Push branch** -- `git push -u origin loom/plan-v5`.
5. **Create PR** -- Use `gh pr create` targeting `main` with a title like "Add LOOM evaluation plan v5: Error Recovery Marathon" and a body summarizing the plan's purpose.
6. **Update STATUS.md** -- Set status to `COMPLETED` and commit.
7. **Write MEMORY.md** -- Document what was done, the PR URL, and any observations.

## Risks

- None significant. This is a straightforward commit-push-PR workflow on an isolated branch with no dependencies.

## Scope

- **In scope**: `tests/loom-eval/plans/plan-v5.md`, `PLAN.md`, `STATUS.md`, `MEMORY.md`
- **Out of scope**: Any other files or implementation work.
