# Plan: Create PR for LOOM Evaluation Plan v19

## Overview

This task commits the evaluation plan file `tests/loom-eval/plans/plan-v19.md` (Workspace Monotonicity Proof) to the `loom/plan-v19` branch, pushes the branch to origin, and creates a GitHub pull request targeting `main`.

## Plan File Summary

Plan v19 tests **Rule 7 (workspace monotonicity)** -- proving that the workspace only moves forward under adversarial conditions. It defines 6 agents:

- 5 provocateur agents that create merge conflicts, validation failures, cascading dependency failures, rebase traps, and post-integration branch advances
- 1 auditor agent that verifies the monotonicity invariant held throughout

The plan specifically targets a contradiction between SKILL.md's `git reset --hard HEAD~1` rollback recipe and Rule 7's forward-only requirement.

## Steps

1. **Stage the plan file**: `git add tests/loom-eval/plans/plan-v19.md`
2. **Commit with LOOM trailers**: Commit with message including `Agent-Id: plan-v19` and `Session-Id: a41180d7-ea25-418a-a022-f9851912c009` trailers.
3. **Push branch**: `git push -u origin loom/plan-v19`
4. **Create PR**: Use `gh pr create` targeting `main` with a descriptive title and body summarizing the plan variant.
5. **Write MEMORY.md**: Document what was done and key findings.
6. **Update STATUS.md**: Set status to `COMPLETED`.
7. **Commit final state**: Commit STATUS.md and MEMORY.md updates.

## Scope

- Only path modified: `tests/loom-eval/plans/plan-v19.md`
- Supporting files: `PLAN.md`, `STATUS.md`, `MEMORY.md` (workflow artifacts in worktree root)

## Risks

- Push may fail if remote branch already exists or SSH access is misconfigured. Mitigation: check remote state before pushing.
- PR creation may fail if branch is not pushed. Mitigation: push before creating PR.

## Dependencies

None.
