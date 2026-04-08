# LOOM Evaluation Plan v4 -- Diamond Dependency

## Goal

Test LOOM's handling of a diamond-shaped dependency DAG: fan-out from a single root agent into two parallel branches, then fan-in to a single leaf agent that depends on both. This exercises topological integration ordering, dependency-aware worktree updates, and the constraint that planning is parallel even when implementation must respect the DAG.

## Diamond DAG

```
        data-model          (A -- no deps)
        /        \
  api-layer    cli-layer    (B, C -- both depend on A)
        \        /
      integration-test      (D -- depends on B and C)
```

The concrete task: build a small key-value store library. Agent A defines the core data model. Agents B and C independently build an HTTP API layer and a CLI layer on top of it. Agent D writes integration tests that exercise both the API and CLI against the shared data model.

## Agent Decomposition

| Agent ID | Role | Task | Dependencies |
|----------|------|------|--------------|
| `data-model` | Foundation | Define a `Store` type with `get`, `set`, `delete`, and `list` operations. Write the module at `src/store.ts` with type exports and an in-memory implementation. Write unit tests at `tests/store.test.ts`. | none |
| `api-layer` | Consumer | Build an HTTP API (Express routes) at `src/api.ts` that imports `Store` from `src/store.ts` and exposes `GET /kv/:key`, `PUT /kv/:key`, `DELETE /kv/:key`, `GET /kv`. Write route tests at `tests/api.test.ts`. | `data-model` |
| `cli-layer` | Consumer | Build a CLI entry point at `src/cli.ts` that imports `Store` from `src/store.ts` and exposes `get <key>`, `set <key> <value>`, `delete <key>`, `list` subcommands. Write CLI tests at `tests/cli.test.ts`. | `data-model` |
| `integration-test` | Verifier | Write integration tests at `tests/integration.test.ts` that start the API server, issue HTTP requests, run CLI commands against the same store, and verify cross-layer consistency. | `api-layer`, `cli-layer` |

## Scopes

| Agent | `paths_allowed` | `paths_denied` |
|-------|-----------------|----------------|
| `data-model` | `src/store.ts`, `tests/store.test.ts` | `[]` |
| `api-layer` | `src/api.ts`, `tests/api.test.ts` | `[]` |
| `cli-layer` | `src/cli.ts`, `tests/cli.test.ts` | `[]` |
| `integration-test` | `tests/integration.test.ts` | `[]` |

No scope overlap. Each agent writes to its own files. The dependency is through imports, not shared file writes.

## Execution Flow

```
Step 1: Create 4 worktrees + branches
        git worktree add .worktrees/data-model        -b loom/data-model
        git worktree add .worktrees/api-layer          -b loom/api-layer
        git worktree add .worktrees/cli-layer          -b loom/cli-layer
        git worktree add .worktrees/integration-test   -b loom/integration-test

Step 2: Write TASK.md + AGENT.json into each worktree. Commit.
        AGENT.json dependencies:
          data-model:       []
          api-layer:        ["data-model"]
          cli-layer:        ["data-model"]
          integration-test: ["api-layer", "cli-layer"]

Step 3: PLANNING PHASE -- spawn ALL 4 agents in parallel
        (Planning does not require deps to be integrated.)
        data-model, api-layer, cli-layer, integration-test all write PLAN.md

Step 4: PLAN GATE -- orchestrator reads all 4 PLAN.md files
        Check for:
        - Scope overlaps (there should be none)
        - api-layer and cli-layer both import from src/store.ts correctly
        - integration-test plan references both api-layer and cli-layer outputs
        - No cycles in the dependency graph (diamond is a DAG, not a cycle)
        Approve or send feedback.

Step 5: IMPLEMENTATION -- Tier 0 (no unmet deps)
        Spawn data-model. Wait for COMPLETED.

Step 6: INTEGRATE data-model
        git merge --no-ff loom/data-model
        Run validation.

Step 7: UPDATE dependent worktrees to include data-model's work
        git -C .worktrees/api-layer merge HEAD
        git -C .worktrees/cli-layer merge HEAD

Step 8: IMPLEMENTATION -- Tier 1 (deps on data-model now met)
        Spawn api-layer and cli-layer in PARALLEL (same message).
        Both can implement concurrently because they have no mutual dependency.
        Wait for both to reach COMPLETED.

Step 9: INTEGRATE api-layer, then cli-layer (order between them is arbitrary)
        git merge --no-ff loom/api-layer
        Run validation.
        git merge --no-ff loom/cli-layer
        Run validation.

Step 10: UPDATE integration-test worktree
         git -C .worktrees/integration-test merge HEAD
         (Now contains data-model + api-layer + cli-layer)

Step 11: IMPLEMENTATION -- Tier 2 (deps on api-layer AND cli-layer now met)
         Spawn integration-test. Wait for COMPLETED.

Step 12: INTEGRATE integration-test
         git merge --no-ff loom/integration-test
         Run validation.

Step 13: Read MEMORY.md from all 4 agents. Clean up all worktrees.
         git worktree remove .worktrees/data-model
         git worktree remove .worktrees/api-layer
         git worktree remove .worktrees/cli-layer
         git worktree remove .worktrees/integration-test
```

## LOOM Features Exercised

| Feature | How |
|---|---|
| Worktree isolation | 4 agents, 4 worktrees, non-overlapping scopes |
| Parallel planning | All 4 agents plan simultaneously (Step 3) |
| Plan gate | Orchestrator reviews 4 plans with cross-plan consistency checks (Step 4) |
| Diamond DAG | A -> {B, C} -> D tests both fan-out and fan-in |
| Topological integration order | Must integrate A before B/C, and B+C before D |
| Dependency-aware worktree update | `git -C .worktrees/<id> merge HEAD` brings integrated work into dependent worktrees (Steps 7, 10) |
| Parallel implementation (mid-DAG) | B and C implement concurrently after A is integrated (Step 8) |
| Sequential implementation (cross-tier) | D waits for both B and C to complete and integrate |
| Commit trailers | Every agent commit must have Agent-Id + Session-Id |
| STATUS.md lifecycle | PLANNING -> IMPLEMENTING -> COMPLETED for all 4 agents |
| MEMORY.md handoff | Each tier's MEMORY.md informs the orchestrator's context for downstream spawns |
| Scope enforcement | Verified at each integration step |
| Worktree cleanup | All 4 removed at end |

## What This Tests Beyond v0

Plan v0 uses a flat fan-in pattern (3 independent agents -> 1 reporter). This plan adds:

| Aspect | v0 | v4 (Diamond) |
|--------|----|----|
| DAG shape | Star (3 -> 1) | Diamond (1 -> 2 -> 1) |
| Tiers of implementation | 2 (parallel, then sequential) | 3 (sequential, parallel, sequential) |
| Worktree updates mid-flow | 0 (eval-report starts from scratch) | 2 rounds (after A, after B+C) |
| Dependency fan-out | No | Yes (A feeds B and C) |
| Dependency fan-in | Yes (3 -> 1) | Yes (B + C -> D) |
| Import-chain correctness | Not applicable (text analysis only) | D imports from modules written by B and C, which import from A |
| Multi-dependency on single agent | Yes (eval-report -> 3 audits) | Split: B depends on A, C depends on A, D depends on B+C |

## Features NOT Tested

- BLOCKED/FAILED states
- Resource limit recovery / continuation agents
- Merge conflict recovery (diamond pattern increases conflict risk but this plan assumes clean scopes)
- Heartbeat enforcement
- Cycle detection (the diamond is a valid DAG; a separate plan should test cycle rejection)
