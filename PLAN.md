# Plan: Create PR for LOOM Evaluation Plan v17

## Overview

This agent's task is to commit the evaluation plan file `tests/loom-eval/plans/plan-v17.md`, push the branch `loom/plan-v17` to origin, and create a GitHub pull request targeting `main`.

The plan file already exists in the worktree. It describes LOOM Evaluation Plan v17, which is a controlled comparison between the full LOOM protocol and the vanilla Agent tool with worktree isolation. The plan defines a word-frequency counter task split across two agents, with a four-dimension measurement framework (overhead, correctness, auditability, error recovery).

## Steps

1. **Stage the plan file** -- `git add tests/loom-eval/plans/plan-v17.md`
2. **Commit with LOOM trailers** -- Commit message describing the plan addition, with `Agent-Id: plan-v17` and `Session-Id: 379f33e9-debc-4e2b-8af6-93e62d65055e` trailers.
3. **Push the branch** -- `git push -u origin loom/plan-v17`
4. **Create the PR** -- Use `gh pr create` targeting `main` with a title and body summarizing the evaluation plan.
5. **Update STATUS.md to COMPLETED** -- Write final status.
6. **Write MEMORY.md** -- Record decisions and findings per LOOM protocol.
7. **Commit final protocol files** -- Commit STATUS.md and MEMORY.md with trailers.

## Scope

- Only path touched outside protocol files: `tests/loom-eval/plans/plan-v17.md`
- No dependencies on other agents.

## Risks

- Push may fail if SSH keys are not configured for the remote. Mitigation: check remote URL and use the bitswell SSH host alias if needed.
- PR creation may fail if `gh` is not authenticated. Mitigation: use `gh auth status` to verify first.
