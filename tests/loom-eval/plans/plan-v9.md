# LOOM Evaluation Plan v9 -- Merge Conflict Gauntlet

## Goal

Stress-test every merge conflict recovery path in the LOOM protocol by deliberately assigning agents overlapping file scopes. The orchestrator knows in advance that integration will produce conflicts. The evaluation measures whether the orchestrator correctly executes all three recovery strategies defined in the protocol (SKILL.md "Error: Merge Conflict" and protocol.md Section 6.3):

1. **Abort** -- `git merge --abort` to keep the workspace clean.
2. **Rebase** -- `git rebase HEAD loom/<id>` to replay agent work on the new workspace HEAD.
3. **Fresh-agent-redo** -- spawn a replacement agent from current workspace HEAD with the failed agent's MEMORY.md as context.

Each recovery path is exercised at least once. The plan also verifies that failed branches are never deleted (30-day retention rule).

---

## Scenario Design

The evaluation uses a synthetic codebase seeded into `tests/loom-eval/conflict-gauntlet/`. Before any agents run, the orchestrator creates these seed files in the workspace:

```
tests/loom-eval/conflict-gauntlet/
  shared/
    config.ts          # Shared config -- will be modified by 3 agents
    utils.ts           # Shared utilities -- will be modified by 2 agents
    types.ts           # Shared type definitions -- will be modified by 2 agents
  module-a/
    index.ts           # Module A entry point
    handler.ts         # Module A handler
  module-b/
    index.ts           # Module B entry point
    handler.ts         # Module B handler
  module-c/
    index.ts           # Module C entry point
    handler.ts         # Module C handler
  integration/
    report.md          # Final integration report
```

Each seed file contains minimal placeholder content (10-20 lines). The files exist so that agents modify them concurrently, guaranteeing textual conflicts.

---

## Agent Decomposition

Seven agents, deliberately designed with overlapping scopes.

| Agent ID | Task Summary | Dependencies |
|----------|-------------|--------------|
| `gantlet-a` | Add logging to module-a AND refactor `shared/config.ts` to add a `logLevel` field | none |
| `gantlet-b` | Add caching to module-b AND refactor `shared/config.ts` to add a `cachePolicy` field AND add cache types to `shared/types.ts` | none |
| `gantlet-c` | Add metrics to module-c AND refactor `shared/config.ts` to add a `metricsEndpoint` field AND add helper to `shared/utils.ts` | none |
| `gantlet-d` | Add error handling utilities to `shared/utils.ts` AND add error types to `shared/types.ts` | none |
| `gantlet-rebase` | (Recovery agent) Rebased redo of whichever agent fails at rebase-recovery | created during recovery |
| `gantlet-redo` | (Recovery agent) Fresh redo from workspace HEAD | created during recovery |
| `gantlet-report` | Read all MEMORY.md files. Compile conflict recovery report. | all others |

The first four agents (`gantlet-a` through `gantlet-d`) run in parallel. They are independent by declaration but their scopes overlap on purpose.

---

## Scope Assignments (Intentional Overlaps)

| Agent | `paths_allowed` | `paths_denied` |
|-------|-----------------|----------------|
| `gantlet-a` | `tests/loom-eval/conflict-gauntlet/module-a/**`, `tests/loom-eval/conflict-gauntlet/shared/config.ts` | `[]` |
| `gantlet-b` | `tests/loom-eval/conflict-gauntlet/module-b/**`, `tests/loom-eval/conflict-gauntlet/shared/config.ts`, `tests/loom-eval/conflict-gauntlet/shared/types.ts` | `[]` |
| `gantlet-c` | `tests/loom-eval/conflict-gauntlet/module-c/**`, `tests/loom-eval/conflict-gauntlet/shared/config.ts`, `tests/loom-eval/conflict-gauntlet/shared/utils.ts` | `[]` |
| `gantlet-d` | `tests/loom-eval/conflict-gauntlet/shared/utils.ts`, `tests/loom-eval/conflict-gauntlet/shared/types.ts` | `[]` |
| `gantlet-rebase` | (inherits from failed agent) | `[]` |
| `gantlet-redo` | (inherits from failed agent) | `[]` |
| `gantlet-report` | `tests/loom-eval/conflict-gauntlet/integration/**` | `[]` |

### Overlap Matrix

This table shows which files are modified by multiple agents, producing guaranteed merge conflicts.

| File | Modified By | Conflict Type |
|------|------------|---------------|
| `shared/config.ts` | `gantlet-a`, `gantlet-b`, `gantlet-c` | **3-way overlap.** Each agent adds a different field to the same config object. After the first integration, the other two will conflict. |
| `shared/utils.ts` | `gantlet-c`, `gantlet-d` | **2-way overlap.** Both add new functions to the same file. |
| `shared/types.ts` | `gantlet-b`, `gantlet-d` | **2-way overlap.** Both add new type definitions to the same file. |

No other files overlap -- each agent's module-specific files (`module-a/`, `module-b/`, `module-c/`) are exclusive.

---

## Execution Flow

### Phase 0: Seed the Workspace

Create the seed files in `tests/loom-eval/conflict-gauntlet/` and commit them. This establishes the base that all agents branch from.

```
Step 0.1: Create seed files with placeholder content
Step 0.2: git add tests/loom-eval/conflict-gauntlet/ && git commit
```

### Phase 1: Create Worktrees (4 parallel agents)

```
git worktree add .worktrees/gantlet-a -b loom/gantlet-a
git worktree add .worktrees/gantlet-b -b loom/gantlet-b
git worktree add .worktrees/gantlet-c -b loom/gantlet-c
git worktree add .worktrees/gantlet-d -b loom/gantlet-d
```

Write TASK.md + AGENT.json into each. Each agent's TASK.md explicitly instructs it to modify the shared files (not just its module files). Commit.

### Phase 2: Planning (parallel)

Spawn all 4 agents in a single message for planning. Each writes PLAN.md + STATUS.md.

### Phase 3: Plan Gate (INTENTIONALLY PERMISSIVE)

The orchestrator reads all 4 PLAN.md files. It detects scope overlaps on `shared/config.ts`, `shared/utils.ts`, and `shared/types.ts`. Normally the plan gate would reject or restructure.

**For this evaluation, the orchestrator deliberately approves all plans despite the overlaps.** The purpose is to observe conflict recovery, not conflict prevention. The orchestrator logs the detected overlaps in a `plan-gate-notes.md` file for the final report.

### Phase 4: Implementation (parallel)

Spawn all 4 agents in a single message. Each implements its work, including modifications to the shared files. All complete successfully in their isolated worktrees (no conflicts during implementation since worktrees are isolated).

### Phase 5: Integration Round 1 -- Clean Merge

Integrate `gantlet-a` first (arbitrary choice -- it touches `config.ts`, `module-a/`).

```bash
git merge --no-ff loom/gantlet-a -m "feat(loom): integrate gantlet-a ..."
# Expected: CLEAN merge. gantlet-a is the first to touch shared/config.ts.
# Run validation.
```

**Checkpoint:** Workspace now contains gantlet-a's version of `shared/config.ts` (with `logLevel` field added).

### Phase 6: Integration Round 2 -- Conflict, Recovery via Abort + Rebase

Attempt to integrate `gantlet-b`:

```bash
git merge --no-ff loom/gantlet-b
# Expected: CONFLICT in shared/config.ts (gantlet-b's config.ts diverges from gantlet-a's)
# May also conflict in shared/types.ts if gantlet-d was integrated first (it won't be yet)
```

**Recovery -- Abort:**

```bash
git merge --abort
# Verify workspace is clean (matches state after gantlet-a integration)
```

**Recovery -- Rebase:**

```bash
git branch loom/gantlet-b-v2 loom/gantlet-b
git rebase HEAD loom/gantlet-b-v2
```

Possible outcomes:
- **Rebase succeeds** (agent's config.ts changes can be replayed on top of gantlet-a's version). Proceed to integrate `loom/gantlet-b-v2`.
- **Rebase conflicts** (the changes are structurally incompatible). Abort rebase with `git rebase --abort`. Proceed to Phase 7 for fresh-agent-redo.

If rebase succeeded:

```bash
git worktree add .worktrees/gantlet-rebase loom/gantlet-b-v2
# Optionally spawn a verification agent to run tests in the rebased worktree
git merge --no-ff loom/gantlet-b-v2 -m "feat(loom): integrate gantlet-b (rebased) ..."
# Run validation
```

**Evaluation assertion:** The original `loom/gantlet-b` branch is NEVER deleted. It is retained as a failed branch.

### Phase 7: Integration Round 3 -- Conflict, Recovery via Fresh-Agent-Redo

Attempt to integrate `gantlet-c`:

```bash
git merge --no-ff loom/gantlet-c
# Expected: CONFLICT in shared/config.ts (now diverges from gantlet-a + gantlet-b combined)
# Expected: Possible conflict in shared/utils.ts if gantlet-d integrated first
```

**Recovery -- Abort:**

```bash
git merge --abort
```

**Recovery -- Fresh Agent Redo:**

Do NOT attempt rebase. Instead, spawn a brand-new agent from the current workspace HEAD. This tests the fresh-agent-redo path (protocol.md Section 6.3, SKILL.md "Error: Merge Conflict" Option B).

```bash
git branch loom/gantlet-redo HEAD
git worktree add .worktrees/gantlet-redo loom/gantlet-redo
```

Write a new TASK.md for `gantlet-redo` that includes:
- The original task from `gantlet-c` (add metrics to module-c, add `metricsEndpoint` to config, add helper to utils)
- The contents of `gantlet-c`'s MEMORY.md (what it learned, what it built)
- Explicit note that `shared/config.ts` and `shared/utils.ts` have been modified by prior agents

Run the standard two-phase cycle (plan, approve, implement) on `gantlet-redo`. Because it branches from the current workspace HEAD (which already has gantlet-a and gantlet-b integrated), its changes will merge cleanly.

```bash
git merge --no-ff loom/gantlet-redo -m "feat(loom): integrate gantlet-redo (redo of gantlet-c) ..."
# Run validation
```

**Evaluation assertion:** The original `loom/gantlet-c` branch is NEVER deleted.

### Phase 8: Integration Round 4 -- Second Overlap File

Integrate `gantlet-d`:

```bash
git merge --no-ff loom/gantlet-d
# Expected: CONFLICT in shared/utils.ts (gantlet-redo already modified it)
# Expected: Possible CONFLICT in shared/types.ts (gantlet-b-v2 already modified it)
```

**Recovery:** Use whichever path has NOT yet been tested in this run. If both abort+rebase and abort+fresh-redo have been exercised, use rebase (it is faster). The goal is to confirm both paths work at least once.

```bash
git merge --abort
git branch loom/gantlet-d-v2 loom/gantlet-d
git rebase HEAD loom/gantlet-d-v2
# If clean: integrate loom/gantlet-d-v2
# If conflict: abort rebase, do fresh-agent-redo
```

**Evaluation assertion:** The original `loom/gantlet-d` branch is NEVER deleted.

### Phase 9: Final Report Agent

All work agents are now integrated. Create and run `gantlet-report`:

```bash
git worktree add .worktrees/gantlet-report -b loom/gantlet-report
```

Its TASK.md instructs it to:
1. Read MEMORY.md from all preceding agents (including failed ones in retained branches).
2. Document each conflict encountered: which files, which agents, what recovery path was used.
3. Write a structured report to `tests/loom-eval/conflict-gauntlet/integration/report.md`.
4. Grade each recovery path (abort, rebase, fresh-redo) on whether the orchestrator followed the protocol correctly.

Run the standard two-phase cycle. Integrate. Clean up all worktrees.

---

## Expected Conflict Map

This table predicts every conflict. Actual results depend on integration order.

| Integration Order | Agent Integrated | Files Conflicting | Recovery Path |
|---|---|---|---|
| 1st | `gantlet-a` | (none -- clean) | N/A |
| 2nd | `gantlet-b` | `shared/config.ts` | **Abort + Rebase** |
| 3rd | `gantlet-c` | `shared/config.ts`, possibly `shared/utils.ts` | **Abort + Fresh-Agent-Redo** |
| 4th | `gantlet-d` | `shared/utils.ts`, `shared/types.ts` | **Abort + Rebase** (or fresh-redo if rebase fails) |
| 5th | `gantlet-report` | (none -- isolated scope) | N/A |

If integration order changes (e.g., `gantlet-d` before `gantlet-c`), the conflict matrix shifts but all three recovery paths are still exercised.

---

## LOOM Features Exercised

| Feature | How |
|---|---|
| Worktree isolation | 7+ worktrees created across the evaluation |
| Parallel planning | 4 agents plan simultaneously |
| Plan gate (with known overlaps) | Orchestrator detects overlaps, logs them, approves anyway |
| Parallel implementation | 4 agents implement simultaneously |
| Commit trailers | Every agent commit has Agent-Id + Session-Id |
| STATUS.md lifecycle | PLANNING -> IMPLEMENTING -> COMPLETED (and FAILED for conflicting agents) |
| MEMORY.md handoff | Failed agent MEMORY.md passed to redo agent via TASK.md |
| Scope enforcement | Verified at integration; overlapping scopes are the point |
| **Merge conflict detection** | Orchestrator detects conflict during `git merge --no-ff` |
| **Abort recovery** | `git merge --abort` restores workspace to pre-merge state |
| **Rebase recovery** | `git rebase HEAD loom/<id>` replays agent work on updated workspace |
| **Fresh-agent-redo recovery** | New agent spawned from workspace HEAD with failed MEMORY.md |
| **Failed branch retention** | Failed branches (`loom/gantlet-b`, `loom/gantlet-c`, `loom/gantlet-d`) never deleted |
| Worktree cleanup | All worktrees removed at end; branches retained |

## Features NOT Tested

- BLOCKED/FAILED states from resource exhaustion (tested by plan-v0 variants)
- Dependency DAG ordering (all 4 work agents are independent)
- Heartbeat enforcement / timeout
- Continuation agents for budget exhaustion
- Scope denial (`paths_denied` is empty for all agents)

---

## Evaluation Criteria (Pass/Fail)

The orchestrator passes this evaluation if ALL of the following hold:

| # | Criterion | Verification |
|---|-----------|--------------|
| 1 | Workspace is never left in a conflicted state | After every `merge --abort`, `git status` shows clean |
| 2 | Abort recovery is used at least once | Orchestrator runs `git merge --abort` before any other recovery |
| 3 | Rebase recovery is used at least once | Orchestrator runs `git rebase HEAD loom/<id>` and integrates the result |
| 4 | Fresh-agent-redo is used at least once | Orchestrator spawns a new agent from workspace HEAD with failed MEMORY.md |
| 5 | Failed branches are never deleted | `git branch --list 'loom/gantlet-*'` shows all original branches at end |
| 6 | Redo agent receives failed agent's MEMORY.md | New TASK.md contains or references the failed agent's findings |
| 7 | All seed files end up with all intended modifications | `shared/config.ts` has logLevel, cachePolicy, metricsEndpoint; `shared/utils.ts` has both additions; `shared/types.ts` has both additions |
| 8 | Final report documents all conflicts and recoveries | `integration/report.md` lists each conflict, files involved, and recovery path used |
| 9 | Scope violations are still checked | Orchestrator verifies file-change scope even when overlaps are expected |
| 10 | Workspace only moves forward (monotonicity) | No `git reset --hard` on the workspace; only `merge --abort` and `merge --no-ff` |
