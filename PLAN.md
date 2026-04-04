# Plan: Create PR for LOOM Evaluation Plan v4

## Objective

Commit the evaluation plan file `tests/loom-eval/plans/plan-v4.md` to branch `loom/plan-v4`, push the branch to origin, and create a GitHub pull request targeting `main`.

## Steps

1. **Stage the plan file** -- `git add tests/loom-eval/plans/plan-v4.md`
2. **Commit with LOOM trailers** -- Commit message includes `Agent-Id: plan-v4` and `Session-Id: 5871a109-1d33-40fd-a2ee-9bd97939d8df` trailers as required by the LOOM protocol.
3. **Push branch** -- `git push -u origin loom/plan-v4`
4. **Create PR** -- Use `gh pr create` targeting `main` with a title describing the plan variant (Diamond Dependency) and a body summarizing what the plan covers.
5. **Write MEMORY.md** -- Record outcome and any observations.
6. **Update STATUS.md** -- Set status to `COMPLETED`.
7. **Commit status files** -- Commit STATUS.md and MEMORY.md with trailers.

## Risks

- Push may fail if remote rejects the branch (e.g., permissions). Mitigation: check SSH config referenced in project memory.
- PR creation may fail if `gh` is not authenticated. Mitigation: `gh` should already be configured in this environment.

## Scope

Only `tests/loom-eval/plans/plan-v4.md` is in the allowed paths for the core deliverable. STATUS.md, PLAN.md, and MEMORY.md are LOOM protocol files written to the worktree root.
