# Plan: Create PR for LOOM Evaluation Plan v12

## Objective

Commit the evaluation plan file `tests/loom-eval/plans/plan-v12.md` to the `loom/plan-v12` branch, push the branch to origin, and create a GitHub pull request targeting `main`.

## Steps

1. **Write PLAN.md and STATUS.md** (this step)
   - Write this plan file describing the approach.
   - Write STATUS.md with `status: PLANNING`.
   - Commit both files with required LOOM trailers (Agent-Id, Session-Id).

2. **Commit the plan file**
   - Stage `tests/loom-eval/plans/plan-v12.md`.
   - Commit with a descriptive message and required LOOM trailers.

3. **Push the branch**
   - Push `loom/plan-v12` to origin with `-u` flag to set upstream tracking.

4. **Create the pull request**
   - Use `gh pr create` targeting `main`.
   - Title: "Add LOOM evaluation plan v12: real-world CI pipeline task"
   - Body: Summary of what plan v12 proposes (a CI validation pipeline for the identity framework), the agent decomposition, and why it tests LOOM well.

5. **Update STATUS.md**
   - Set status to `COMPLETED`.
   - Write MEMORY.md with required sections.
   - Commit final status update.

## Constraints

- Work only in the `/home/willem/bitswell/bitswell/.worktrees/plan-v12` worktree.
- Only modify allowed paths: `tests/loom-eval/plans/plan-v12.md`, plus LOOM protocol files (STATUS.md, PLAN.md, MEMORY.md).
- All commits must include `Agent-Id: plan-v12` and `Session-Id: 21685c1c-2a44-4b52-9588-f3186aa11891` trailers.
- Token budget: 50,000. Timeout: 300s.

## Risk Assessment

- Low risk: the task is straightforward file commit, push, and PR creation.
- The plan file already exists in the worktree; no generation needed.
- Only dependency is network access to push and create the PR.
