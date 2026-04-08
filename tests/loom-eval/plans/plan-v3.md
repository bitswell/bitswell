# LOOM Evaluation Plan v3 -- Deep Dependency Chain

## Goal

Stress-test LOOM's handling of sequential dependencies by building a layered TypeScript module where each layer genuinely depends on the output of the previous one. This forces the orchestrator to execute a 5-agent linear chain (A -> B -> C -> D -> E), exercising topological ordering, worktree update propagation between tiers, and incremental integration after every step.

## Scenario: Layered Event Processing Pipeline

Build a simple event processing pipeline in `tests/loom-eval/pipeline/`. Each layer exports types and functions consumed by the next layer. The chain cannot be parallelized without breaking real import dependencies.

```
schema-layer -> transport-layer -> processor-layer -> router-layer -> api-layer
```

| Layer | Produces | Consumes |
|-------|----------|----------|
| `schema-layer` | `types.ts` -- Event, Metadata, Priority enums, type guards | nothing |
| `transport-layer` | `transport.ts` -- EventBus class with pub/sub, serialization | Event type from schema-layer |
| `processor-layer` | `processor.ts` -- EventProcessor with transform/filter/validate | EventBus from transport-layer, Event from schema-layer |
| `router-layer` | `router.ts` -- EventRouter with pattern matching and routing table | EventProcessor from processor-layer, EventBus from transport-layer |
| `api-layer` | `index.ts` -- Public API facade, re-exports, createPipeline() factory | All of the above |

## Agent Decomposition

| Agent ID | Task | Dependencies | Tier |
|----------|------|--------------|------|
| `schema-layer` | Define the core event type system: Event interface, Metadata interface, Priority enum, type guard functions (isEvent, isHighPriority). Export everything from `types.ts`. Include unit tests in `types.test.ts`. | none | 0 |
| `transport-layer` | Implement EventBus class: subscribe/unsubscribe by event type, publish with async dispatch, serialize/deserialize events to JSON. Import Event from schema-layer. Include unit tests. | `schema-layer` | 1 |
| `processor-layer` | Implement EventProcessor: transform (map event fields), filter (predicate-based), validate (using type guards from schema-layer). Accept EventBus in constructor for input. Include unit tests. | `transport-layer` | 2 |
| `router-layer` | Implement EventRouter: register routes as pattern/handler pairs, match events by type+priority pattern, dispatch to handlers via processor. Include unit tests. | `processor-layer` | 3 |
| `api-layer` | Create the public facade: re-export key types, provide createPipeline() factory that wires up all layers, write an integration test exercising the full chain end-to-end. | `router-layer` | 4 |

Dependency DAG (linear chain):

```
schema-layer --> transport-layer --> processor-layer --> router-layer --> api-layer
  (tier 0)         (tier 1)            (tier 2)           (tier 3)        (tier 4)
```

## Scopes

| Agent | `paths_allowed` | `paths_denied` |
|-------|-----------------|----------------|
| `schema-layer` | `tests/loom-eval/pipeline/types.ts`, `tests/loom-eval/pipeline/types.test.ts` | `[]` |
| `transport-layer` | `tests/loom-eval/pipeline/transport.ts`, `tests/loom-eval/pipeline/transport.test.ts` | `[]` |
| `processor-layer` | `tests/loom-eval/pipeline/processor.ts`, `tests/loom-eval/pipeline/processor.test.ts` | `[]` |
| `router-layer` | `tests/loom-eval/pipeline/router.ts`, `tests/loom-eval/pipeline/router.test.ts` | `[]` |
| `api-layer` | `tests/loom-eval/pipeline/index.ts`, `tests/loom-eval/pipeline/index.test.ts` | `[]` |

No scope overlap -- each agent writes exactly two files (implementation + test).

## Execution Flow

```
Step 1: Create 5 worktrees + branches
        git worktree add .worktrees/schema-layer     -b loom/schema-layer
        git worktree add .worktrees/transport-layer   -b loom/transport-layer
        git worktree add .worktrees/processor-layer   -b loom/processor-layer
        git worktree add .worktrees/router-layer      -b loom/router-layer
        git worktree add .worktrees/api-layer         -b loom/api-layer

Step 2: Write TASK.md + AGENT.json into each worktree. Commit.

Step 3: PLANNING PHASE -- spawn ALL 5 agents in parallel
        Planning does not require dependencies to be integrated (per protocol).
        All 5 agents write PLAN.md simultaneously.

Step 4: PLAN GATE -- orchestrator reads all 5 PLAN.md files
        Verify:
          - Each agent's planned exports match the next agent's planned imports.
          - Interface contracts are compatible across the chain.
          - No scope overlaps.
          - Estimated effort is reasonable.
        Approve or append ## Feedback and re-plan.

Step 5: IMPLEMENTATION TIER 0 -- spawn schema-layer (no deps)
        Wait for COMPLETED status.

Step 6: INTEGRATE schema-layer
        git merge --no-ff loom/schema-layer
        Validate: types.ts exports Event, Metadata, Priority, isEvent, isHighPriority.

Step 7: UPDATE transport-layer worktree
        git -C .worktrees/transport-layer merge HEAD
        This gives transport-layer access to the integrated schema types.

Step 8: IMPLEMENTATION TIER 1 -- spawn transport-layer
        Tell it: "schema-layer is integrated. Event type available at
        tests/loom-eval/pipeline/types.ts."
        Wait for COMPLETED status.

Step 9: INTEGRATE transport-layer
        git merge --no-ff loom/transport-layer
        Validate: transport.ts imports from types.ts, exports EventBus.

Step 10: UPDATE processor-layer worktree
         git -C .worktrees/processor-layer merge HEAD

Step 11: IMPLEMENTATION TIER 2 -- spawn processor-layer
         Tell it: "schema-layer and transport-layer are integrated.
         EventBus at transport.ts, Event at types.ts."
         Wait for COMPLETED status.

Step 12: INTEGRATE processor-layer
         git merge --no-ff loom/processor-layer
         Validate: processor.ts imports from transport.ts and types.ts.

Step 13: UPDATE router-layer worktree
         git -C .worktrees/router-layer merge HEAD

Step 14: IMPLEMENTATION TIER 3 -- spawn router-layer
         Tell it: "All upstream layers integrated. EventProcessor at
         processor.ts, EventBus at transport.ts."
         Wait for COMPLETED status.

Step 15: INTEGRATE router-layer
         git merge --no-ff loom/router-layer
         Validate: router.ts imports from processor.ts and transport.ts.

Step 16: UPDATE api-layer worktree
         git -C .worktrees/api-layer merge HEAD

Step 17: IMPLEMENTATION TIER 4 -- spawn api-layer
         Tell it: "All 4 upstream layers are integrated. Build the
         public facade and integration test."
         Wait for COMPLETED status.

Step 18: INTEGRATE api-layer
         git merge --no-ff loom/api-layer
         Validate: index.ts imports from all layers, integration test passes.

Step 19: Read MEMORY.md from all 5 agents. Clean up all worktrees.
```

## Why the Chain Cannot Be Collapsed

Each layer has a genuine code dependency on the layer before it:

- `transport-layer` imports `Event` from `schema-layer` to type its pub/sub methods.
- `processor-layer` imports `EventBus` from `transport-layer` for its input source and `Event` from `schema-layer` for its transform signatures.
- `router-layer` imports `EventProcessor` from `processor-layer` for dispatch.
- `api-layer` imports from all four layers to wire them together.

An agent cannot write correct, type-safe code against interfaces that do not yet exist. The planning phase can run in parallel because plans only describe intent -- but implementation must be strictly sequential.

## Worktree Update Pattern

After each integration, the next agent's worktree must be updated so it can see the newly integrated code:

```bash
# After integrating tier N, update tier N+1 worktree:
git -C .worktrees/<next-agent> merge HEAD
```

This is the `git -C .worktrees/<dep-id> merge HEAD` pattern from the LOOM skill's "Agents with Dependencies" recipe. The plan exercises it 4 times (once per tier transition), making it the most-tested single operation in this evaluation.

## LOOM Features Exercised

| Feature | How | Frequency |
|---------|-----|-----------|
| Worktree isolation | 5 agents, 5 worktrees, non-overlapping scopes | 5x |
| Parallel planning | All 5 agents plan simultaneously despite sequential deps | 1x (5 agents) |
| Plan gate with cross-agent contract validation | Orchestrator checks that exports/imports align across the chain | 1x (5 plans) |
| Topological integration ordering | Strict tier-by-tier: 0, 1, 2, 3, 4. No out-of-order integration possible. | 5x |
| Worktree update after integration | `git -C ... merge HEAD` propagates integrated code to next tier | 4x |
| Sequential implementation spawning | Each implementation spawn waits for the prior tier's integration | 5x |
| Dependency declaration in AGENT.json | Each agent (except tier 0) declares exactly one dependency | 4x |
| Commit trailers (Agent-Id + Session-Id) | Every agent commit across all 5 agents | ~15-25 commits |
| STATUS.md lifecycle | PLANNING -> IMPLEMENTING -> COMPLETED, repeated 5 times | 5x |
| MEMORY.md handoff | Each agent's findings inform the next tier's TASK.md context | 4x |
| Scope enforcement at integration | Verified 5 times, once per merge | 5x |
| Worktree cleanup | All 5 removed at end | 5x |

## Features NOT Tested

- Parallel implementation (by design -- this plan is purely sequential)
- BLOCKED/FAILED states
- Resource limit recovery / continuation agents
- Merge conflict recovery
- Heartbeat enforcement
- Diamond or fan-out dependency shapes

## Key Evaluation Questions

1. **Does the orchestrator correctly refuse to spawn tier N+1 before tier N is integrated?** The protocol says "agents with unmet deps wait." A naive orchestrator might try to parallelize.
2. **Does the worktree update (`git -C ... merge HEAD`) actually make the integrated code visible?** If the merge is a no-op or the worktree is stale, the next agent will fail on imports.
3. **Does the plan gate catch interface mismatches?** If `schema-layer` plans to export `EventData` but `transport-layer` plans to import `Event`, the orchestrator should catch this before any implementation begins.
4. **Does integration ordering hold under 5 tiers?** The original plan (v0) only tested 2 tiers (3 parallel -> 1 dependent). Five sequential tiers stress the topological sort more thoroughly.
5. **Do commit trailers accumulate correctly across a long chain?** After integrating all 5, the workspace history should contain commits from 5 distinct Agent-Ids, each with unique Session-Ids.
