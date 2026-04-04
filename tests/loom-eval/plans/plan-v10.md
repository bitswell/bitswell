# LOOM Evaluation Plan v10 -- Continuation Agent Test

## Goal

End-to-end test of the **resource-limit recovery recipe** (SKILL.md "Error: Resource Limit", protocol Section 10.3, Example 4). An agent is given a token budget so small it *must* exhaust 90% before finishing, forcing the full BLOCKED -> continuation-agent flow.

This plan deliberately targets the features that v0 listed under "Features NOT Tested": BLOCKED state, resource-limit recovery, and continuation agents.

## Design Rationale

The task assigned to the primary agent is intentionally large relative to its budget. The agent must:

1. Begin implementation and make partial progress.
2. Hit the 90% budget threshold.
3. Execute the checkpoint procedure: write MEMORY.md with remaining work, set STATUS.md to BLOCKED with `blocked_reason: resource_limit`, commit, and exit.
4. The orchestrator then reads MEMORY.md, spawns a continuation agent branching from the blocked agent's branch, writes a reduced TASK.md, and runs the standard two-phase cycle on the continuation.

The task itself is real work -- generating a set of structured markdown files -- so the agents produce verifiable artifacts at each stage. The evaluator can confirm partial output from the first agent and completed output from the continuation.

## Agent Decomposition

| Agent ID | Role | Token Budget | Task | Dependencies |
|----------|------|-------------|------|-------------|
| `catalog-writer` | Primary worker | **5,000** (impossibly small) | Write a catalog of 20 items across 4 files, each with structured metadata | none |
| `catalog-writer-cont` | Continuation worker | **100,000** (generous) | Complete the remaining catalog items from where `catalog-writer` stopped | `catalog-writer` (branches from it) |
| `catalog-verifier` | Verifier | **50,000** | Read all 4 catalog files and verify completeness, structural correctness, and that the continuation preserved the primary agent's partial work | `catalog-writer-cont` |

Note: `catalog-writer-cont` is NOT declared upfront. It is created by the orchestrator after `catalog-writer` goes BLOCKED. This tests the dynamic agent creation path.

## Scopes

| Agent | `paths_allowed` | `paths_denied` |
|-------|-----------------|----------------|
| `catalog-writer` | `tests/loom-eval/catalog/**` | `[]` |
| `catalog-writer-cont` | `tests/loom-eval/catalog/**` | `[]` |
| `catalog-verifier` | `tests/loom-eval/verification/**` | `[]` |

The primary and continuation agents share the same scope -- this is correct because the continuation branches from the primary and works in the same domain. The verifier writes to a separate directory.

## Detailed Execution Flow

### Phase 0: Setup

```
Step 0.1: Create output directories in the workspace
          mkdir -p tests/loom-eval/catalog
          mkdir -p tests/loom-eval/verification

Step 0.2: Create worktree for catalog-writer
          git worktree add .worktrees/catalog-writer -b loom/catalog-writer
```

### Phase 1: Assign the Doomed Agent

```
Step 1.1: Write TASK.md into .worktrees/catalog-writer/

          The task: "Create 4 catalog files (catalog-a.md through catalog-d.md)
          in tests/loom-eval/catalog/. Each file must contain 5 items. Each item
          has: name, category, description (2-3 sentences), tags (3+), and a
          cross-reference to one item in another file. Total: 20 items across
          4 files."

          This is far too much work for a 5,000-token budget.

Step 1.2: Write AGENT.json with the critical fields:
          {
            "agent_id": "catalog-writer",
            "session_id": "<uuid>",
            "protocol_version": "loom/1",
            "context_window_tokens": 200000,
            "token_budget": 5000,
            "dependencies": [],
            "scope": {
              "paths_allowed": ["tests/loom-eval/catalog/**"],
              "paths_denied": []
            },
            "timeout_seconds": 300
          }

          The 5,000 token_budget is the key. 90% = 4,500 tokens.
          The agent will exhaust this before completing even one catalog file.

Step 1.3: Commit TASK.md + AGENT.json to the catalog-writer branch.
```

### Phase 2: Planning -- Primary Agent

```
Step 2.1: Spawn catalog-writer via Agent tool for PLANNING phase.
          Use the worker-template.md with substituted placeholders.
          Append: "This is your PLANNING phase. Read TASK.md and AGENT.json.
          Write PLAN.md. Update STATUS.md to PLANNING. Commit both. Then
          return. Do NOT implement."

Step 2.2: Read PLAN.md from .worktrees/catalog-writer/PLAN.md.
          The plan should acknowledge the tiny budget and note the risk
          of resource exhaustion.

Step 2.3: PLAN GATE -- Approve the plan.
          Append "## Feedback\n\nApproved." to TASK.md. Commit.
```

### Phase 3: Implementation -- Primary Agent (Expected to BLOCK)

```
Step 3.1: Spawn catalog-writer for IMPLEMENTATION phase.
          Append: "This is your IMPLEMENTATION phase. Your plan was approved.
          Read PLAN.md, implement the work, write MEMORY.md, set STATUS.md
          to COMPLETED, commit, and return."

Step 3.2: Agent returns. The orchestrator MUST now verify:

          CHECK A -- STATUS.md has status: BLOCKED
            head -20 .worktrees/catalog-writer/STATUS.md
            Expect: status: BLOCKED
            Expect: blocked_reason: resource_limit

          CHECK B -- MEMORY.md exists and contains the three required sections
            cat .worktrees/catalog-writer/MEMORY.md
            Expect: ## Key Findings (with partial progress notes)
            Expect: ## Decisions
            Expect: ## Deviations from Plan
            The "Key Findings" section MUST describe what was completed
            and what remains.

          CHECK C -- Partial work artifacts exist
            ls tests/loom-eval/catalog/ (inside the worktree)
            Expect: at least some catalog files or partial content.
            It is acceptable if no catalog files were created (the agent
            may have hit the limit during STATUS/MEMORY writing). The
            critical check is that MEMORY.md documents what remains.

          If STATUS.md shows COMPLETED instead of BLOCKED, the agent
          somehow finished in budget -- the test premise is invalid.
          Increase the task size or decrease the budget and re-run.

          If STATUS.md shows FAILED (not BLOCKED), check the error
          category. If it is resource_limit, that is acceptable -- the
          agent used FAILED instead of BLOCKED. Note this as a protocol
          deviation but continue with the recovery flow.
```

### Phase 4: Orchestrator Recovery -- Spawn Continuation Agent

This is the core of what is being tested. The orchestrator follows the "Error: Resource Limit" recipe from SKILL.md.

```
Step 4.1: Read MEMORY.md from the blocked agent.
          Extract: which catalog files/items were completed, which remain.

Step 4.2: Create continuation worktree branching from the blocked agent's branch.
          git worktree add .worktrees/catalog-writer-cont \
            -b loom/catalog-writer-cont loom/catalog-writer

          This ensures the continuation agent starts with the primary
          agent's partial commits in its history.

Step 4.3: Write a NEW TASK.md into .worktrees/catalog-writer-cont/
          covering ONLY the remaining work. Example:

          "# Task: Complete catalog (continuation)

          ## Objective
          Complete the catalog files started by agent catalog-writer.
          That agent was blocked by resource_limit after partial progress.

          ## Context
          Prior agent's MEMORY.md reported:
          <paste Key Findings from the blocked agent's MEMORY.md>

          Existing partial files in tests/loom-eval/catalog/ are already
          committed. Do NOT overwrite completed items. Add only the
          missing items and files.

          ## Scope
          - Allowed paths: tests/loom-eval/catalog/**
          - Denied paths: []

          ## Acceptance Criteria
          - [ ] All 4 catalog files exist (catalog-a.md through catalog-d.md)
          - [ ] Each file has exactly 5 items with all required fields
          - [ ] Cross-references between files are valid
          - [ ] Previously written items are preserved unchanged

          ## Dependencies
          - Continues from catalog-writer (branch loom/catalog-writer)

          ## Constraints
          - Token budget: 100000
          - Timeout: 600"

Step 4.4: Write AGENT.json for catalog-writer-cont:
          {
            "agent_id": "catalog-writer-cont",
            "session_id": "<new-uuid>",
            "protocol_version": "loom/1",
            "context_window_tokens": 200000,
            "token_budget": 100000,
            "dependencies": [],
            "scope": {
              "paths_allowed": ["tests/loom-eval/catalog/**"],
              "paths_denied": []
            },
            "timeout_seconds": 600
          }

          Note: dependencies is [] because this is a continuation, not a
          DAG dependency. The continuation branches from the blocked agent's
          branch, so it inherits the work directly.

Step 4.5: Commit TASK.md + AGENT.json to the catalog-writer-cont branch.
```

### Phase 5: Planning -- Continuation Agent

```
Step 5.1: Spawn catalog-writer-cont for PLANNING phase.
Step 5.2: Read PLAN.md. Verify it references the prior agent's partial work.
Step 5.3: PLAN GATE -- Approve.
```

### Phase 6: Implementation -- Continuation Agent

```
Step 6.1: Spawn catalog-writer-cont for IMPLEMENTATION phase.
Step 6.2: On return, verify:

          CHECK D -- STATUS.md has status: COMPLETED
          CHECK E -- All 4 catalog files exist in tests/loom-eval/catalog/
          CHECK F -- Each file has 5 items with required fields
          CHECK G -- MEMORY.md documents the continuation, references
                     the prior agent's findings
          CHECK H -- Items written by catalog-writer are preserved
                     (not overwritten or lost)
```

### Phase 7: Integrate Continuation Agent

```
Step 7.1: Verify scope compliance:
          git -C .worktrees/catalog-writer-cont diff --name-only \
            $(git -C .worktrees/catalog-writer-cont merge-base HEAD loom/catalog-writer)
          All changed files must match tests/loom-eval/catalog/**

Step 7.2: Merge into workspace:
          git merge --no-ff loom/catalog-writer-cont -m "feat(catalog): complete catalog (continuation)
          Agent-Id: orchestrator
          Session-Id: <orchestrator-session-id>"

          Note: We do NOT integrate the blocked agent's branch separately.
          The continuation branch already contains the blocked agent's commits.
```

### Phase 8: Verification Agent

```
Step 8.1: Create worktree:
          git worktree add .worktrees/catalog-verifier -b loom/catalog-verifier

Step 8.2: Write TASK.md for catalog-verifier:
          "Verify the catalog at tests/loom-eval/catalog/. Check:
          (1) All 4 files exist. (2) Each has 5 items. (3) Each item has
          name, category, description, tags, cross-reference. (4) All
          cross-references point to real items. Write a verification
          report to tests/loom-eval/verification/report.md."

Step 8.3: Write AGENT.json (budget 50000, deps [], scope verification/**)
Step 8.4: Commit. Run two-phase cycle (plan, gate, implement).
Step 8.5: Integrate. Read verification report.
```

### Phase 9: Cleanup

```
Step 9.1: Read MEMORY.md from all agents (catalog-writer, catalog-writer-cont,
          catalog-verifier). Compile orchestrator summary.

Step 9.2: Remove worktrees:
          git worktree remove .worktrees/catalog-writer
          git worktree remove .worktrees/catalog-writer-cont
          git worktree remove .worktrees/catalog-verifier

Step 9.3: Retain branches:
          loom/catalog-writer       -- BLOCKED, retained per spec (30 days)
          loom/catalog-writer-cont  -- COMPLETED, may be cleaned
          loom/catalog-verifier     -- COMPLETED, may be cleaned
```

## Assertions -- What the Evaluation Must Verify

The evaluation passes only if ALL of the following hold:

### Resource Limit Behavior (Primary Agent)

| # | Assertion | Source |
|---|-----------|--------|
| A1 | `catalog-writer` STATUS.md has `status: BLOCKED` (or `FAILED` with `category: resource_limit`) | Protocol Section 10.3 |
| A2 | `blocked_reason` is `resource_limit` (if BLOCKED) | Worker template, Budget Reservation section |
| A3 | MEMORY.md exists with all 3 required sections | Protocol Section 4.2 |
| A4 | MEMORY.md describes what was completed and what remains | Worker template, Budget Reservation: "clear summary of what remains to be done" |
| A5 | Final commit includes Agent-Id and Session-Id trailers | Level 1 Rule 2 |

### Continuation Agent Spawning (Orchestrator)

| # | Assertion | Source |
|---|-----------|--------|
| B1 | Continuation branch was created from the blocked agent's branch, not from workspace HEAD | SKILL.md recipe: `git worktree add ... loom/<id>` |
| B2 | Continuation TASK.md references the prior agent's MEMORY.md findings | SKILL.md recipe: "Reference prior MEMORY.md findings" |
| B3 | Continuation TASK.md covers only the remaining work, not the full original task | SKILL.md recipe: "covering only the remaining work" |
| B4 | Continuation agent received a fresh session_id (different from the blocked agent) | schemas.md: "unique per invocation" |

### Continuation Agent Completion

| # | Assertion | Source |
|---|-----------|--------|
| C1 | `catalog-writer-cont` STATUS.md has `status: COMPLETED` | Lifecycle state machine |
| C2 | All 4 catalog files exist with 5 items each (20 total) | Task acceptance criteria |
| C3 | Items written by the blocked agent are preserved (not overwritten) | Continuation semantics |
| C4 | MEMORY.md references the prior agent and continuation context | Protocol Section 4.2 |
| C5 | All commits have Agent-Id and Session-Id trailers | Level 1 Rule 2 |

### Integration and Verification

| # | Assertion | Source |
|---|-----------|--------|
| D1 | Only the continuation branch is merged (not the blocked branch separately) | The continuation already contains the blocked agent's commits |
| D2 | Merge is `--no-ff` with orchestrator trailers | SKILL.md command patterns |
| D3 | Blocked agent's branch is retained (not deleted) | Protocol Section 6.3 |
| D4 | Verification report confirms structural completeness | Acceptance criteria |

## LOOM Features Exercised

| Feature | How |
|---------|-----|
| Worktree isolation | 3 agents, 3 worktrees |
| Two-phase lifecycle | All agents go through plan then implement |
| Plan gate | Orchestrator reviews all plans before implementation |
| BLOCKED state | Primary agent enters BLOCKED via resource_limit |
| `blocked_reason` field | Set to `resource_limit` in STATUS.md |
| Budget reservation (90% rule) | Primary agent must detect 90% usage and checkpoint |
| MEMORY.md as recovery checkpoint | Blocked agent writes progress; continuation reads it |
| Continuation agent branching | `git worktree add ... -b loom/<id>-cont loom/<id>` |
| Dynamic agent creation | Continuation agent not declared upfront |
| Reduced TASK.md for continuation | New task covers only remaining work |
| Branch retention for failed/blocked agents | Blocked branch preserved |
| Scope enforcement at integration | Verified for continuation agent |
| Commit trailers | Every commit checked for Agent-Id + Session-Id |
| STATUS.md lifecycle | PLANNING -> IMPLEMENTING -> BLOCKED (primary), PLANNING -> IMPLEMENTING -> COMPLETED (continuation) |

## Features NOT Tested

- Parallel agents (covered by v0)
- Dependency DAG ordering (covered by v0)
- Merge conflict recovery
- FAILED state (only tangentially -- if primary agent uses FAILED instead of BLOCKED)
- Heartbeat enforcement / timeout termination
- Agent-to-agent MEMORY.md reads (peer reads)

## Known Risks

1. **Agent ignores budget.** If the Agent tool implementation does not actually enforce token counting, the worker may not recognize it has hit 90%. Mitigation: the orchestrator can simulate budget exhaustion by setting `token_budget: 5000` which is less than what a single planning phase consumes. Even if the agent does not self-checkpoint, the Agent tool itself may terminate the session, at which point the orchestrator treats the unclean exit as a resource_limit scenario.

2. **Agent completes in budget.** If the task is somehow completable in 5,000 tokens, the BLOCKED path is never triggered. Mitigation: the task requires generating substantial structured content (20 items with cross-references). If this still completes, reduce the budget further or add more items.

3. **Continuation agent overwrites partial work.** The continuation should append, not replace. Mitigation: TASK.md explicitly says "Do NOT overwrite completed items" and CHECK H validates preservation.
