# Plan: Commit and PR for LOOM Evaluation Plan v11 (Schema Fuzzing)

## Approach

The plan file `tests/loom-eval/plans/plan-v11.md` already exists in the worktree. The task is to commit it with proper LOOM trailers, push the `loom/plan-v11` branch to origin, and create a GitHub pull request targeting `main`.

## Steps

1. Stage `tests/loom-eval/plans/plan-v11.md` for commit.
2. Commit with a descriptive message and required LOOM trailers (`Agent-Id: plan-v11`, `Session-Id: 9b8206ae-b6bf-4a57-852b-866fe9a5e862`).
3. Push branch `loom/plan-v11` to origin with `-u` flag.
4. Create a PR via `gh pr create` targeting `main` with a title and body describing the plan variant (schema fuzzing with 141 test cases across 8 agents).
5. Update STATUS.md to COMPLETED and write MEMORY.md with required sections.
6. Commit final status files with LOOM trailers.

## Files to Modify

- `tests/loom-eval/plans/plan-v11.md` (commit existing file, no modifications)
- `STATUS.md` (create/update with status transitions)
- `PLAN.md` (this file)
- `MEMORY.md` (create with required sections)

## Risks

- Push may fail if remote rejects the branch (e.g., permissions). Mitigation: use the configured SSH key for the bitswell account.
- PR creation may fail if `gh` is not authenticated. Mitigation: `gh` should already be configured in this environment.

## Estimated Effort

- Tokens: ~5000
- Files: 4
