# Plan: Create PR for LOOM Evaluation Plan v7

## Summary

Commit the evaluation plan file `tests/loom-eval/plans/plan-v7.md` to the `loom/plan-v7` branch, push the branch to origin, and create a pull request targeting `main`.

## Steps

1. **Stage the plan file.** Add `tests/loom-eval/plans/plan-v7.md` to the git index.

2. **Commit with LOOM trailers.** Create a commit with a descriptive message and the required `Agent-Id: plan-v7` and `Session-Id: 265f767b-3d82-48fd-adb9-ce8aca3773e1` trailers.

3. **Push the branch.** Push `loom/plan-v7` to origin with `-u` to set upstream tracking.

4. **Create the pull request.** Use `gh pr create` targeting `main` with:
   - Title: "Add LOOM evaluation plan v7: Self-Improvement"
   - Body describing the plan variant (5 agents coordinate to find and fix issues in the LOOM skill files themselves, producing concrete patches rather than reports)

5. **Update STATUS.md to COMPLETED.** Write final status with the PR URL.

6. **Write MEMORY.md.** Summarize what was done for future reference.

7. **Commit status and memory files.** Final commit with LOOM trailers.

## Risks

- Push may fail if the remote rejects the branch (unlikely for a new branch).
- PR creation may fail if `gh` is not authenticated (will check and handle).

## Scope Compliance

Only modifying files within the allowed scope:
- `tests/loom-eval/plans/plan-v7.md` (the deliverable)
- `PLAN.md`, `STATUS.md`, `MEMORY.md` (LOOM protocol files)
