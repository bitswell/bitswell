# Plan: Create PR for LOOM Evaluation Plan v14

## Summary

This task commits the LOOM evaluation plan v14 (Security Red Team) to the `loom/plan-v14` branch, pushes it to origin, and opens a pull request targeting `main`.

Plan v14 is a security red-teaming evaluation that probes LOOM protocol trust boundaries using adversarial agents. It covers five attack surfaces: worktree escape, scope violation, feedback injection, identity forgery, and cross-agent writes, with a reporter agent to compile findings.

## Steps

1. **Stage the plan file** -- `git add tests/loom-eval/plans/plan-v14.md`
2. **Commit with LOOM trailers** -- Commit message includes `Agent-Id: plan-v14` and `Session-Id: 35425016-7290-4c4d-8bd0-f8bc06dae00e` trailers.
3. **Push the branch** -- `git push -u origin loom/plan-v14`
4. **Create the PR** -- Use `gh pr create` targeting `main` with a descriptive title and body summarizing the security red team evaluation plan.
5. **Update STATUS.md** -- Set status to COMPLETED.
6. **Write MEMORY.md** -- Record what was done and the PR URL.

## Risks

- None significant. This is a straightforward commit-push-PR workflow for a single markdown file.
