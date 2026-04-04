# Plan: Create PR for LOOM Evaluation Plan v15

## Summary

Commit the evaluation plan file `tests/loom-eval/plans/plan-v15.md` (an incremental complexity ladder for LOOM skill evaluation) to the `loom/plan-v15` branch, push the branch to origin, and create a GitHub pull request targeting `main`.

## Steps

1. **Write PLAN.md and STATUS.md** (this step) -- Create the planning artifacts required by the LOOM protocol. STATUS.md starts as `status: PLANNING`.

2. **Commit planning artifacts** -- Stage PLAN.md and STATUS.md and commit with the required LOOM trailers (`Agent-Id: plan-v15`, `Session-Id: a12395ad-6c27-4cec-9a96-f86ddeeec378`).

3. **Commit plan file** -- Stage `tests/loom-eval/plans/plan-v15.md` and commit with a descriptive message and LOOM trailers.

4. **Push branch** -- Push `loom/plan-v15` to origin with `-u` to set upstream tracking.

5. **Create PR** -- Use `gh pr create` targeting `main` with a title like "Add LOOM evaluation plan v15: incremental complexity ladder" and a body summarizing the plan's four-level approach (L1 single agent, L2 parallel pair, L3 sequential dependency, L4 diamond DAG).

6. **Update STATUS.md** -- Set status to `COMPLETED` and write MEMORY.md with required sections.

7. **Final commit** -- Commit the updated STATUS.md and MEMORY.md with LOOM trailers.

## Scope

Only `tests/loom-eval/plans/plan-v15.md` is in the allowed paths for implementation. PLAN.md, STATUS.md, and MEMORY.md are LOOM protocol artifacts.

## Risks

- Push may fail if origin rejects the branch (unlikely for a new branch).
- PR creation may fail if `gh` is not authenticated (will check and handle).
