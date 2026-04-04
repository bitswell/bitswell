# Plan: Create PR for LOOM Evaluation Plan v8

## Summary

Commit the evaluation plan file `tests/loom-eval/plans/plan-v8.md` to branch `loom/plan-v8`, push the branch to origin, and create a GitHub pull request targeting `main`.

The plan file describes a LOOM evaluation that maps the bitswell agent team (vesper, ratchet, moss, drift, sable, thorn, glitch, bitswelt) onto LOOM worker roles to test whether the uniform LOOM protocol can accommodate agents with fundamentally different working styles.

## Steps

1. **Stage the plan file** -- `git add tests/loom-eval/plans/plan-v8.md`
2. **Commit with LOOM trailers** -- Commit message includes `Agent-Id: plan-v8` and `Session-Id: b34989d7-e7ef-48f3-9c1f-bbae1006faf4` trailers.
3. **Push branch** -- `git push -u origin loom/plan-v8`
4. **Create PR** -- `gh pr create` targeting `main` with a descriptive title and body summarizing the plan variant.
5. **Update STATUS.md** -- Set status to `COMPLETED`.
6. **Write MEMORY.md** -- Record key decisions and outcome.
7. **Final commit** -- Commit STATUS.md and MEMORY.md updates.

## Risks

- Push may fail if remote rejects the branch (e.g., permissions). Mitigation: use the configured SSH key for the bitswell account.
- PR creation may fail if `gh` is not authenticated. Mitigation: check `gh auth status` before attempting.

## Estimated token usage

Well under the 50,000 token budget. This is a straightforward commit-push-PR task.
