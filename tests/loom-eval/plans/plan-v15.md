# LOOM Evaluation Plan v15 -- Incremental Complexity Ladder

## Goal

Evaluate the LOOM skill at `.claude/skills/loom/` by running four sequential tests of increasing complexity. Each level builds on the confidence gained by the previous one. A failure at any level halts the ladder -- there is no point testing a diamond DAG if a single agent cannot complete the lifecycle.

## Design Rationale

The original plan (v0) jumps directly to a 4-agent topology with parallel independent agents plus a dependent reporter. This exercises many features at once, but if something fails, diagnosing the root cause is difficult: was it the plan gate? Scope enforcement? Dependency ordering? Worktree isolation? Parallel spawning?

This plan isolates complexity dimensions. Each level adds exactly one new coordination mechanism on top of the previous level. If level N fails, the failure is attributable to the mechanism introduced at that level, not to mechanisms already validated at levels 1 through N-1.

## Ladder Overview

| Level | Topology | Agents | New mechanism tested |
|-------|----------|--------|----------------------|
| L1 | Single | 1 | Full lifecycle: worktree, plan gate, STATUS.md, MEMORY.md, commit trailers, scope, integration, cleanup |
| L2 | Parallel pair | 2 | Parallel Agent tool spawns (same message), scope overlap check at plan gate, integration order irrelevance |
| L3 | Sequential dependency | 2 | `dependencies` in AGENT.json, topological integration order, worktree merge to propagate dependency |
| L4 | Diamond DAG | 4 | Multi-tier dependency graph, mixed parallel/sequential phases, full protocol exercise |

## Halt Condition

If any level produces a FAILED agent, a scope violation at integration, a merge conflict, or a STATUS.md that does not parse as valid YAML, the ladder halts. The orchestrator writes a `LADDER-RESULT.md` with the level reached and the failure details.

---

## Level 1: Single Agent

### Purpose

Validate the complete LOOM lifecycle end-to-end with the simplest possible topology: one orchestrator, one worker, one worktree. Every protocol file format (TASK.md, AGENT.json, PLAN.md, STATUS.md, MEMORY.md) is exercised. Every state transition (PLANNING -> IMPLEMENTING -> COMPLETED) is verified.

### Agent Decomposition

| Agent ID | Task | Dependencies |
|----------|------|-------------|
| `l1-file-audit` | Read all 5 LOOM skill files. Produce a checklist of every MUST/MUST NOT requirement found. Write the checklist to `tests/loom-eval/l1-file-audit/requirements.md`. | none |

### Scope

| Agent | `paths_allowed` | `paths_denied` |
|-------|-----------------|----------------|
| `l1-file-audit` | `tests/loom-eval/l1-file-audit/**` | `[]` |

### Execution Flow

```
Step 1: Create worktree
        git worktree add .worktrees/l1-file-audit -b loom/l1-file-audit

Step 2: Write TASK.md + AGENT.json into worktree. Commit.

Step 3: PLANNING PHASE -- spawn l1-file-audit
        Agent returns with PLAN.md + STATUS.md committed.

Step 4: PLAN GATE -- orchestrator reads PLAN.md
        Single agent: no overlap check needed.
        Verify plan references the correct skill files.
        Approve or send feedback.

Step 5: IMPLEMENTATION PHASE -- re-spawn l1-file-audit
        Agent reads skill files, extracts requirements, writes checklist.

Step 6: VALIDATE before integration:
        - STATUS.md has status: COMPLETED (valid YAML)
        - MEMORY.md has all 3 required sections
        - Every commit has Agent-Id + Session-Id trailers
        - Only files within scope were modified
        - requirements.md exists in output directory

Step 7: INTEGRATE
        git merge --no-ff loom/l1-file-audit

Step 8: Clean up worktree. Record L1 PASS.
```

### Validation Checklist (orchestrator verifies)

- [ ] Worktree created and isolated
- [ ] TASK.md + AGENT.json committed by orchestrator
- [ ] PLAN.md written during planning phase
- [ ] STATUS.md transitions: PLANNING -> IMPLEMENTING -> COMPLETED
- [ ] All commits have Agent-Id and Session-Id trailers
- [ ] MEMORY.md has Key Findings, Decisions, Deviations from Plan sections
- [ ] Agent only modified files within `tests/loom-eval/l1-file-audit/**`
- [ ] Merge into workspace succeeds without conflict
- [ ] Worktree removed after integration

### LOOM Features Exercised

| Feature | How |
|---------|-----|
| Worktree isolation | Single worktree, non-overlapping scope |
| Two-phase lifecycle | Plan then implement |
| Plan gate | Orchestrator reviews 1 plan |
| Commit trailers | Agent-Id + Session-Id on every commit |
| STATUS.md lifecycle | PLANNING -> IMPLEMENTING -> COMPLETED |
| MEMORY.md | Written with all 3 sections |
| Scope enforcement | Verified at integration |
| Worktree cleanup | Removed after merge |

---

## Level 2: Parallel Pair

### Purpose

Introduce the first multi-agent coordination: two agents with no dependencies between them. This tests the orchestrator's ability to spawn multiple Agent tool calls in a single message (parallel execution), check for scope overlaps at the plan gate, and integrate in arbitrary order.

### Agent Decomposition

| Agent ID | Task | Dependencies |
|----------|------|-------------|
| `l2-schema-check` | Read `references/schemas.md`. For each file format (TASK.md, AGENT.json, STATUS.md, MEMORY.md, PLAN.md, commit messages, branch naming), verify the schema is internally consistent. Write findings to `tests/loom-eval/l2-schema-check/findings.md`. | none |
| `l2-example-check` | Read `references/examples.md`. For each of the 5 worked examples, verify that the git commands shown are syntactically correct and that the workflow matches the protocol described in SKILL.md. Write findings to `tests/loom-eval/l2-example-check/findings.md`. | none |

### Scope

| Agent | `paths_allowed` | `paths_denied` |
|-------|-----------------|----------------|
| `l2-schema-check` | `tests/loom-eval/l2-schema-check/**` | `[]` |
| `l2-example-check` | `tests/loom-eval/l2-example-check/**` | `[]` |

No scope overlap between agents.

### Execution Flow

```
Step 1: Create 2 worktrees
        git worktree add .worktrees/l2-schema-check  -b loom/l2-schema-check
        git worktree add .worktrees/l2-example-check -b loom/l2-example-check

Step 2: Write TASK.md + AGENT.json into each worktree. Commit each.

Step 3: PLANNING PHASE -- spawn BOTH agents in a single message (parallel)
        Both return with PLAN.md + STATUS.md committed.

Step 4: PLAN GATE -- orchestrator reads both PLAN.md files
        Check for scope overlaps: l2-schema-check writes to l2-schema-check/,
        l2-example-check writes to l2-example-check/. No overlap.
        Verify neither plan strays outside its allowed paths.
        Approve both.

Step 5: IMPLEMENTATION PHASE -- spawn BOTH agents in a single message (parallel)
        Both do analysis and write findings.

Step 6: VALIDATE both agents:
        For each agent:
        - STATUS.md has status: COMPLETED
        - MEMORY.md has all 3 required sections
        - All commits have trailers
        - Files only within scope
        - findings.md exists

Step 7: INTEGRATE in any order (no dependencies)
        git merge --no-ff loom/l2-schema-check
        (run validation)
        git merge --no-ff loom/l2-example-check
        (run validation)

Step 8: Clean up both worktrees. Record L2 PASS.
```

### Validation Checklist (orchestrator verifies)

- [ ] Both worktrees created and isolated from each other
- [ ] Parallel planning: both agents spawned in same message
- [ ] Plan gate: scope overlap check performed (and confirmed no overlap)
- [ ] Parallel implementation: both agents spawned in same message
- [ ] Both STATUS.md files show COMPLETED
- [ ] Both MEMORY.md files have all 3 sections
- [ ] All commits on both branches have Agent-Id + Session-Id trailers
- [ ] Neither agent modified files outside its scope
- [ ] Both merges succeed without conflict
- [ ] Integration order was arbitrary (no dependency constraint)
- [ ] Both worktrees removed

### LOOM Features Exercised (new at this level)

| Feature | How |
|---------|-----|
| Parallel planning spawn | 2 Agent calls in one message |
| Parallel implementation spawn | 2 Agent calls in one message |
| Plan gate scope overlap check | Orchestrator verifies no path collisions |
| Order-independent integration | Either agent can merge first |

---

## Level 3: Sequential Dependency

### Purpose

Introduce the dependency mechanism. Two agents where the second depends on the first. This tests: `dependencies` array in AGENT.json, topological integration order (the upstream agent MUST be integrated before the downstream agent is spawned for implementation), and the worktree merge pattern that propagates integrated work into the dependent agent's worktree.

Planning is still parallel (per protocol: planning does not require dependencies to be integrated). Only implementation respects the dependency order.

### Agent Decomposition

| Agent ID | Task | Dependencies |
|----------|------|-------------|
| `l3-rule-extract` | Read SKILL.md and `references/protocol.md`. Extract every numbered rule, non-negotiable rule, and MUST/MUST NOT statement. Write a structured JSON array to `tests/loom-eval/l3-rule-extract/rules.json` where each entry has `{id, source_file, section, text, level}`. | none |
| `l3-rule-coverage` | Read `rules.json` produced by `l3-rule-extract`. For each rule, check whether `references/examples.md` contains at least one example that exercises that rule. Write a coverage matrix to `tests/loom-eval/l3-rule-coverage/coverage.md` listing covered and uncovered rules. | `l3-rule-extract` |

### Scope

| Agent | `paths_allowed` | `paths_denied` |
|-------|-----------------|----------------|
| `l3-rule-extract` | `tests/loom-eval/l3-rule-extract/**` | `[]` |
| `l3-rule-coverage` | `tests/loom-eval/l3-rule-coverage/**` | `[]` |

No scope overlap.

### Execution Flow

```
Step 1: Create 2 worktrees
        git worktree add .worktrees/l3-rule-extract  -b loom/l3-rule-extract
        git worktree add .worktrees/l3-rule-coverage -b loom/l3-rule-coverage

Step 2: Write TASK.md + AGENT.json into each worktree.
        l3-rule-coverage AGENT.json: "dependencies": ["l3-rule-extract"]
        Commit each.

Step 3: PLANNING PHASE -- spawn BOTH agents in parallel
        (Planning does not require deps to be integrated.)
        Both return with PLAN.md + STATUS.md.

Step 4: PLAN GATE -- orchestrator reads both PLAN.md files
        Verify l3-rule-extract will produce rules.json at the expected path.
        Verify l3-rule-coverage's plan references rules.json as input.
        Check the dependency chain: coverage depends on extract.
        No scope overlap. Approve both.

Step 5: IMPLEMENTATION PHASE -- sequential, respecting dependencies

  Step 5a: Spawn l3-rule-extract (no unmet deps)
           Agent reads skill files, extracts rules, writes rules.json.

  Step 5b: Validate l3-rule-extract:
           - STATUS.md = COMPLETED
           - MEMORY.md valid
           - Commit trailers present
           - Scope respected
           - rules.json exists and is valid JSON

  Step 5c: Integrate l3-rule-extract into workspace
           git merge --no-ff loom/l3-rule-extract
           (run validation)

  Step 5d: Update l3-rule-coverage worktree with integrated work
           git -C .worktrees/l3-rule-coverage merge HEAD
           (This brings rules.json into the coverage agent's worktree)

  Step 5e: Spawn l3-rule-coverage (deps now met)
           Agent reads rules.json, cross-references examples, writes coverage.md.

Step 6: Validate l3-rule-coverage:
        - STATUS.md = COMPLETED
        - MEMORY.md valid
        - Commit trailers present
        - Scope respected
        - coverage.md exists

Step 7: Integrate l3-rule-coverage
        git merge --no-ff loom/l3-rule-coverage

Step 8: Clean up both worktrees. Record L3 PASS.
```

### Validation Checklist (orchestrator verifies)

- [ ] Parallel planning: both agents plan concurrently despite dependency
- [ ] l3-rule-extract implemented and integrated before l3-rule-coverage starts implementation
- [ ] Worktree merge pattern used: `git -C .worktrees/l3-rule-coverage merge HEAD`
- [ ] l3-rule-coverage successfully reads rules.json from its worktree after merge
- [ ] Topological integration order respected: extract before coverage
- [ ] AGENT.json for l3-rule-coverage contains `"dependencies": ["l3-rule-extract"]`
- [ ] Both STATUS.md files show COMPLETED
- [ ] All commits on both branches have trailers
- [ ] Neither agent modified files outside scope

### LOOM Features Exercised (new at this level)

| Feature | How |
|---------|-----|
| Dependency declaration | `dependencies` array in AGENT.json |
| Parallel planning with deps | Both plan in parallel; deps only affect implementation |
| Topological implementation order | Extract must complete before coverage starts |
| Worktree merge for dependency propagation | Integrated work pulled into dependent worktree |
| Sequential implementation spawn | Coverage waits for extract integration |

---

## Level 4: Diamond DAG

### Purpose

Exercise the full protocol with a diamond-shaped dependency graph: two independent middle-tier agents that both depend on a single upstream agent, and a downstream agent that depends on both middle-tier agents. This is the canonical complex topology that tests all coordination mechanisms simultaneously.

```
        l4-inventory
        /          \
  l4-gap-analysis   l4-consistency
        \          /
        l4-report
```

### Agent Decomposition

| Agent ID | Task | Dependencies |
|----------|------|-------------|
| `l4-inventory` | Read all 5 LOOM skill files. Produce a structured inventory of all protocol concepts: states, transitions, operations, file formats, rules, error categories, security boundaries. Write to `tests/loom-eval/l4-inventory/inventory.json`. | none |
| `l4-gap-analysis` | Read `inventory.json` from l4-inventory. For each protocol concept, check whether it is fully specified (has definition, constraints, examples, error handling). Flag gaps. Write to `tests/loom-eval/l4-gap-analysis/gaps.md`. | `l4-inventory` |
| `l4-consistency` | Read `inventory.json` from l4-inventory. Cross-check all 5 skill files for contradictions: same concept defined differently, conflicting rules, mismatched field names or types. Write to `tests/loom-eval/l4-consistency/contradictions.md`. | `l4-inventory` |
| `l4-report` | Read `gaps.md` from l4-gap-analysis and `contradictions.md` from l4-consistency. Compile a unified evaluation report with severity ratings (critical/major/minor) and recommended fixes. Write to `tests/loom-eval/l4-report/eval-report.md`. | `l4-gap-analysis`, `l4-consistency` |

### Scope

| Agent | `paths_allowed` | `paths_denied` |
|-------|-----------------|----------------|
| `l4-inventory` | `tests/loom-eval/l4-inventory/**` | `[]` |
| `l4-gap-analysis` | `tests/loom-eval/l4-gap-analysis/**` | `[]` |
| `l4-consistency` | `tests/loom-eval/l4-consistency/**` | `[]` |
| `l4-report` | `tests/loom-eval/l4-report/**` | `[]` |

No scope overlap across any agents.

### Dependency DAG (topological tiers)

```
Tier 0 (no deps):        l4-inventory
Tier 1 (depends on T0):  l4-gap-analysis, l4-consistency    [parallel]
Tier 2 (depends on T1):  l4-report
```

### Execution Flow

```
Step 1: Create 4 worktrees
        git worktree add .worktrees/l4-inventory    -b loom/l4-inventory
        git worktree add .worktrees/l4-gap-analysis -b loom/l4-gap-analysis
        git worktree add .worktrees/l4-consistency  -b loom/l4-consistency
        git worktree add .worktrees/l4-report       -b loom/l4-report

Step 2: Write TASK.md + AGENT.json into each worktree
        AGENT.json dependencies:
          l4-inventory:    []
          l4-gap-analysis: ["l4-inventory"]
          l4-consistency:  ["l4-inventory"]
          l4-report:       ["l4-gap-analysis", "l4-consistency"]
        Verify DAG is acyclic before committing.
        Commit each.

Step 3: PLANNING PHASE -- spawn ALL 4 agents in a single message (parallel)
        Planning does not require dependencies to be integrated.
        All 4 return with PLAN.md + STATUS.md committed.

Step 4: PLAN GATE -- orchestrator reads all 4 PLAN.md files
        Check for scope overlaps across all 4 agents (none expected).
        Verify dependency chain coherence:
          - l4-gap-analysis and l4-consistency both expect inventory.json
          - l4-report expects gaps.md and contradictions.md
        Verify no agent plans to modify files outside its scope.
        Approve all 4 (or send feedback and re-plan as needed).

Step 5: IMPLEMENTATION -- Tier 0
        Spawn l4-inventory (no unmet deps).
        Agent reads all skill files, produces inventory.json.

Step 6: Validate + integrate l4-inventory
        - STATUS.md = COMPLETED
        - MEMORY.md valid, all 3 sections
        - Commit trailers on every commit
        - Scope: only l4-inventory/** modified
        - inventory.json exists and is valid JSON
        git merge --no-ff loom/l4-inventory
        (run validation)

Step 7: Propagate to Tier 1 worktrees
        git -C .worktrees/l4-gap-analysis merge HEAD
        git -C .worktrees/l4-consistency merge HEAD
        (Both worktrees now contain inventory.json)

Step 8: IMPLEMENTATION -- Tier 1
        Spawn l4-gap-analysis AND l4-consistency in a single message (parallel).
        Both read inventory.json and produce their respective outputs.

Step 9: Validate + integrate Tier 1 (either order, no dep between them)

  Step 9a: Validate l4-gap-analysis
           - STATUS.md = COMPLETED
           - MEMORY.md, trailers, scope all verified
           - gaps.md exists
           git merge --no-ff loom/l4-gap-analysis
           (run validation)

  Step 9b: Validate l4-consistency
           - STATUS.md = COMPLETED
           - MEMORY.md, trailers, scope all verified
           - contradictions.md exists
           git merge --no-ff loom/l4-consistency
           (run validation)

Step 10: Propagate to Tier 2 worktree
         git -C .worktrees/l4-report merge HEAD
         (l4-report worktree now contains gaps.md and contradictions.md)

Step 11: IMPLEMENTATION -- Tier 2
         Spawn l4-report (all deps met).
         Agent reads gaps.md and contradictions.md, compiles report.

Step 12: Validate + integrate l4-report
         - STATUS.md = COMPLETED
         - MEMORY.md valid
         - Commit trailers present
         - Scope respected
         - eval-report.md exists
         git merge --no-ff loom/l4-report
         (run validation)

Step 13: Clean up all 4 worktrees. Record L4 PASS.
```

### Validation Checklist (orchestrator verifies)

- [ ] 4 worktrees created, all isolated
- [ ] DAG validated as acyclic before any spawns
- [ ] All 4 agents planned in parallel (single message)
- [ ] Plan gate reviewed all 4 plans, checked scope overlaps
- [ ] Tier 0: l4-inventory implemented and integrated first
- [ ] Tier 1: both agents received propagated workspace via worktree merge
- [ ] Tier 1: both agents spawned in parallel (single message)
- [ ] Tier 1: integrated in arbitrary order (no mutual dependency)
- [ ] Tier 2: l4-report received both Tier 1 outputs via worktree merge
- [ ] Tier 2: l4-report spawned only after both Tier 1 agents integrated
- [ ] All 4 STATUS.md files show COMPLETED
- [ ] All 4 MEMORY.md files have all 3 required sections
- [ ] Every commit across all 4 branches has Agent-Id + Session-Id trailers
- [ ] No agent modified files outside its scope
- [ ] All 4 merges succeeded without conflict
- [ ] All 4 worktrees removed

### LOOM Features Exercised (new at this level)

| Feature | How |
|---------|-----|
| Diamond DAG | 4 agents, 3 tiers, fan-out and fan-in |
| Multi-dependency | l4-report depends on 2 agents |
| Mixed parallel/sequential implementation | Tier 1 parallel, tiers 0 and 2 sequential |
| Multi-tier worktree propagation | Workspace merged into worktrees at two different tiers |
| 4-agent plan gate | Scope overlap check across 4 agents simultaneously |

---

## Cumulative Feature Coverage

| LOOM Feature | L1 | L2 | L3 | L4 |
|---|---|---|---|---|
| Worktree isolation | X | X | X | X |
| Two-phase lifecycle (plan/implement) | X | X | X | X |
| Plan gate | X | X | X | X |
| Commit trailers (Agent-Id, Session-Id) | X | X | X | X |
| STATUS.md lifecycle | X | X | X | X |
| MEMORY.md with 3 sections | X | X | X | X |
| Scope enforcement | X | X | X | X |
| Worktree cleanup | X | X | X | X |
| Parallel planning spawn | -- | X | X | X |
| Parallel implementation spawn | -- | X | -- | X |
| Plan gate scope overlap check (multi-agent) | -- | X | X | X |
| Order-independent integration | -- | X | -- | X |
| Dependency declaration in AGENT.json | -- | -- | X | X |
| Topological implementation order | -- | -- | X | X |
| Worktree merge for dependency propagation | -- | -- | X | X |
| Diamond DAG | -- | -- | -- | X |
| Multi-dependency (2+ deps on one agent) | -- | -- | -- | X |
| Mixed parallel/sequential implementation | -- | -- | -- | X |
| Multi-tier propagation | -- | -- | -- | X |
| 4-agent plan gate | -- | -- | -- | X |

## Features NOT Tested

- BLOCKED/FAILED states (agent error paths)
- Resource limit recovery / continuation agents
- Merge conflict recovery
- Heartbeat enforcement and stale agent detection
- Budget reservation at 90% threshold
- Level 2 protocol features (content-addressed memory refs)
- Cross-agent MEMORY.md reading during implementation

These omitted features are better served by a dedicated error-path evaluation plan, not a complexity ladder focused on coordination topology.

## Output Artifacts

On successful completion of all 4 levels, the following files will exist:

```
tests/loom-eval/
  l1-file-audit/requirements.md        -- MUST/MUST NOT checklist
  l2-schema-check/findings.md          -- Schema consistency findings
  l2-example-check/findings.md         -- Example correctness findings
  l3-rule-extract/rules.json           -- Structured rule inventory
  l3-rule-coverage/coverage.md         -- Rule-to-example coverage matrix
  l4-inventory/inventory.json          -- Protocol concept inventory
  l4-gap-analysis/gaps.md              -- Specification gap analysis
  l4-consistency/contradictions.md     -- Cross-document contradiction check
  l4-report/eval-report.md            -- Unified evaluation with severity ratings
  LADDER-RESULT.md                     -- Final pass/fail with level reached
```

Each artifact builds on the analysis from earlier levels, providing progressively deeper evaluation of the LOOM skill while simultaneously stress-testing LOOM's own coordination mechanisms.
