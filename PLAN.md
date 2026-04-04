# Plan: Create PR for LOOM Evaluation Plan v13

## Objective

Commit the evaluation plan file `tests/loom-eval/plans/plan-v13.md` to the `loom/plan-v13` branch, push the branch to origin, and create a GitHub pull request targeting `main`.

## Steps

1. **Stage the plan file**: Add `tests/loom-eval/plans/plan-v13.md` to the git index.

2. **Commit with LOOM trailers**: Create a commit with a descriptive message and the required `Agent-Id: plan-v13` and `Session-Id: 57623fa1-033d-4d6e-9bf1-2b1ad08bc92c` trailers.

3. **Push the branch**: Push `loom/plan-v13` to `origin` with the `-u` flag to set up tracking.

4. **Create the PR**: Use `gh pr create` targeting `main` with:
   - Title: `Add LOOM evaluation plan v13: Observability Audit`
   - Body summarizing the plan's goal (auditing LOOM's runtime observability artifacts), the 5 agents it defines (heartbeat-enforcer, status-parser, git-trail-auditor, branch-retention, operator-dashboard), and the LOOM features exercised.

5. **Update STATUS.md**: Set status to `COMPLETED` after the PR is created.

6. **Write MEMORY.md**: Record the PR URL, branch name, and any issues encountered.

7. **Final commit**: Commit STATUS.md and MEMORY.md updates with LOOM trailers.

## Scope

- Only file being committed to the repo: `tests/loom-eval/plans/plan-v13.md`
- STATUS.md, PLAN.md, and MEMORY.md are worktree-local coordination files.

## Risks

- Push may fail if the remote rejects the branch (permissions, branch protection). Mitigation: verify SSH access before pushing.
- PR creation may fail if `gh` is not authenticated. Mitigation: check `gh auth status` first.

## Estimated Token Usage

Low -- this is a straightforward commit-push-PR workflow with no implementation work.
