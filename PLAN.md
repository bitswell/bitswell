# Plan: Create PR for LOOM Evaluation Plan v20

## Overview

Commit the evaluation plan file `tests/loom-eval/plans/plan-v20.md` (already present in the worktree), push the branch `loom/plan-v20` to origin, and create a GitHub pull request targeting `main`.

## Steps

### Step 1: Commit the plan file

- Stage `tests/loom-eval/plans/plan-v20.md`
- Commit with message describing the plan variant (Cross-Repo Portability evaluation)
- Include required LOOM trailers: `Agent-Id: plan-v20` and `Session-Id: a079eeb1-4eb2-43f6-b637-331fdaf476e3`

### Step 2: Push the branch

- Push `loom/plan-v20` to origin with `-u` to set upstream tracking

### Step 3: Create the pull request

- Target branch: `main`
- Title: concise, under 70 characters, describing plan-v20 content
- Body: summary of what the plan evaluates (cross-repo portability), the agent decomposition (3 environment testers + 1 reporter), and key evaluation areas (empty repo, monorepo, submodules)

### Step 4: Update STATUS.md to COMPLETED and write MEMORY.md

- Set STATUS.md status to `COMPLETED` with the PR URL
- Write MEMORY.md with required sections summarizing what was done

## Risks

- Push may fail if SSH key is not configured for the `origin` remote. Mitigation: check remote URL and use the correct SSH alias if needed.
- PR creation may fail if `gh` CLI is not authenticated. Mitigation: verify `gh auth status` before creating PR.

## Estimated Token Usage

Low -- this is a mechanical commit/push/PR task with no code generation.
