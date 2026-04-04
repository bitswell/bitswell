# LOOM Evaluation Plan v17 -- Comparison with Native Agent Tool

## Goal

Determine what LOOM adds over the bare Claude Code `Agent` tool with `isolation:worktree`. Run the **same task** twice under controlled conditions -- once using the full LOOM protocol (orchestrator, TASK.md, AGENT.json, PLAN.md, STATUS.md, MEMORY.md, commit trailers, scope enforcement, plan gate, dependency-ordered integration) and once using the vanilla Agent tool with worktree isolation and a plain-English prompt. Compare overhead, correctness, auditability, and error recovery.

## Why This Variant

The native Agent tool already provides filesystem isolation via `isolation:worktree`. A skeptic would ask: does LOOM's ceremony -- five protocol files, YAML front matter, two-phase spawning, commit trailers, plan gates, scope validation -- actually improve outcomes, or is it overhead that burns tokens for no measurable gain? This plan produces evidence either way.

## Task Selection

The task must be complex enough to stress coordination but simple enough that correctness is objectively verifiable. We use a **two-agent task with a dependency**: write a small utility library (`lib`) and a consumer that imports it (`app`).

**Concrete task:** Create a word-frequency counter.

- **Agent A (lib)**: Create `src/wordfreq/counter.ts` exporting a `countWords(text: string): Map<string, number>` function. Handle punctuation stripping, case normalization, and empty input. Write unit tests in `tests/wordfreq/counter.test.ts`.
- **Agent B (app)**: Create `src/wordfreq/cli.ts` that imports `countWords`, reads a text file from argv, and prints the top-N words. Write integration tests in `tests/wordfreq/cli.test.ts`. Depends on Agent A being integrated first.

This task exercises: worktree isolation, parallel planning, sequential implementation (dependency), cross-agent handoff, file creation, and test validation.

## Experimental Design

### Arm 1: LOOM Protocol (Full Ceremony)

Follow SKILL.md exactly. Two-phase spawn, all protocol files, commit trailers, plan gate, scope enforcement, dependency-ordered integration.

| Agent ID | Scope (allowed) | Scope (denied) | Dependencies |
|----------|-----------------|----------------|--------------|
| `wf-lib` | `src/wordfreq/counter.*`, `tests/wordfreq/counter.*`, `tests/loom-eval/v17-loom/**` | `src/wordfreq/cli.*` | none |
| `wf-app` | `src/wordfreq/cli.*`, `tests/wordfreq/cli.*`, `tests/loom-eval/v17-loom/**` | `src/wordfreq/counter.*` | `wf-lib` |

Execution sequence (mirrors SKILL.md core flow):

```
 1. Create worktrees:
      git worktree add .worktrees/wf-lib -b loom/wf-lib
      git worktree add .worktrees/wf-app -b loom/wf-app

 2. Write TASK.md + AGENT.json for each. Commit.

 3. PLANNING PHASE: Spawn both agents in parallel.
      Both write PLAN.md + STATUS.md (PLANNING). Both return.

 4. PLAN GATE: Orchestrator reads both PLAN.md files.
      - Verify wf-lib exports a function signature wf-app can consume.
      - Verify no scope overlap.
      - Approve or append feedback and re-plan.

 5. IMPLEMENTATION PHASE (sequential due to dependency):
      a. Spawn wf-lib. Wait for COMPLETED.
      b. Validate: check STATUS.md, verify scope compliance, merge --no-ff.
      c. Update wf-app worktree: git -C .worktrees/wf-app merge HEAD
      d. Spawn wf-app. Wait for COMPLETED.
      e. Validate: check STATUS.md, verify scope compliance, merge --no-ff.

 6. Run tests from workspace: npx vitest (or equivalent).

 7. Read MEMORY.md from both agents. Record findings.

 8. Clean up worktrees.
```

### Arm 2: Vanilla Agent Tool (No LOOM)

Use the Agent tool with `isolation:worktree` and a natural-language prompt. No protocol files, no two-phase spawn, no plan gate, no commit trailers, no scope enforcement. The agent is told what to build and left to figure out how.

Execution sequence:

```
 1. Spawn Agent A with prompt:
      "Create src/wordfreq/counter.ts exporting countWords(text: string): Map<string, number>.
       Handle punctuation, case normalization, empty input.
       Write tests in tests/wordfreq/counter.test.ts.
       Commit your work and return."

 2. Wait for Agent A to return. Inspect its branch. Merge into workspace.

 3. Spawn Agent B with prompt:
      "Create src/wordfreq/cli.ts importing countWords from src/wordfreq/counter.
       Read a file path from argv, print top-N words.
       Write tests in tests/wordfreq/cli.test.ts.
       Commit your work and return."

 4. Wait for Agent B to return. Inspect its branch. Merge into workspace.

 5. Run tests from workspace.

 6. Record observations.
```

No PLAN.md review step. No AGENT.json. No STATUS.md. No MEMORY.md. No commit trailers. No scope validation. The orchestrator just spawns, waits, merges.

## Measurement Framework

### M1: Overhead

Measured per arm. Lower is better for overhead; the question is whether LOOM's overhead buys enough in other dimensions.

| Metric | How measured | LOOM source | Vanilla source |
|--------|-------------|-------------|----------------|
| **Total commits** | `git log --oneline \| wc -l` on each agent branch | Agent branch | Agent branch |
| **Protocol-only commits** | Commits touching only STATUS.md, PLAN.md, MEMORY.md, AGENT.json, TASK.md | Count from LOOM branch | 0 (by definition) |
| **Orchestrator actions** | Count of distinct bash commands the orchestrator runs | Tally during execution | Tally during execution |
| **Wall-clock time** | Start-to-finish for each arm | Timestamp logs | Timestamp logs |
| **Agent spawns** | Number of Agent tool invocations | 4 (2 plan + 2 impl) | 2 |

### M2: Correctness

Both arms must produce the same functional result. Measured objectively.

| Metric | How measured |
|--------|-------------|
| **Tests pass** | `npx vitest --reporter=json` exit code + pass/fail counts |
| **Function signature match** | Does `countWords` have the specified signature? AST inspection or grep. |
| **CLI behavior** | Run cli.ts on a sample text file; compare output against expected top-N. |
| **Scope violations** | (LOOM only) Did any agent commit files outside its allowed scope? |

### M3: Auditability

Can a human reviewer reconstruct what happened, why, and who did what?

| Metric | How measured | LOOM | Vanilla |
|--------|-------------|------|---------|
| **Commit attribution** | Do commits identify which agent made them? | Yes: Agent-Id + Session-Id trailers | Maybe: depends on agent behavior |
| **Plan review trail** | Is there a record of what was planned vs. what was implemented? | PLAN.md + MEMORY.md (Deviations section) | No structured record |
| **Decision rationale** | Can you find *why* a design choice was made? | MEMORY.md Decisions section | Possibly in commit messages, if agent bothers |
| **State history** | Can you reconstruct the agent's lifecycle? | STATUS.md + git log with trailer filter | git log only |
| **Scope proof** | Can you verify the agent stayed in its lane? | AGENT.json scope + orchestrator validation | No mechanism |

Scoring: For each metric, rate 0 (absent), 1 (partial/implicit), 2 (explicit/structured). Sum per arm.

### M4: Error Recovery

Intentionally break things and observe recovery behavior.

| Scenario | How introduced | LOOM expected behavior | Vanilla expected behavior |
|----------|---------------|----------------------|--------------------------|
| **Bad plan** | Orchestrator rejects wf-app PLAN.md at plan gate (appends feedback, re-plans) | Agent re-reads TASK.md with feedback, rewrites PLAN.md, orchestrator approves round 2 | N/A -- no plan gate exists. Error surfaces at implementation time or never. |
| **Scope violation** | Manually edit wf-lib to also create cli.ts (outside scope) | Orchestrator rejects at integration (scope check) | Merge succeeds; violation undetected |
| **Merge conflict** | After integrating wf-lib, manually edit counter.ts in workspace before integrating wf-app | LOOM: merge --abort, rebase or spawn fresh agent per protocol | Vanilla: merge fails, orchestrator must improvise |
| **Resource exhaustion** | Give wf-lib a very low token_budget in AGENT.json | Agent hits 90%, writes MEMORY.md checkpoint, sets BLOCKED. Orchestrator spawns continuation. | Agent runs until it finishes or crashes. No structured checkpoint. |

For each scenario, record: (a) was the error detected? (b) was recovery attempted? (c) did recovery succeed? (d) was the error documented in a way a human can review later?

## Agent Decomposition (Meta-Level)

This evaluation itself is orchestrated by LOOM (eating our own dogfood). Three agents run the experiment; a fourth compiles results.

| Agent ID | Role | Task | Dependencies |
|----------|------|------|--------------|
| `v17-loom-arm` | Executor | Run Arm 1 (full LOOM). Record all M1-M4 measurements in `tests/loom-eval/v17-loom/measurements.md`. | none |
| `v17-vanilla-arm` | Executor | Run Arm 2 (vanilla Agent). Record all M1-M4 measurements in `tests/loom-eval/v17-vanilla/measurements.md`. | none |
| `v17-error-suite` | Tester | Run all four M4 error scenarios against both arms. Record in `tests/loom-eval/v17-errors/results.md`. | `v17-loom-arm`, `v17-vanilla-arm` |
| `v17-report` | Reporter | Read MEMORY.md from all three agents + measurements files. Compile comparative analysis in `tests/loom-eval/v17-report/comparison.md`. | `v17-loom-arm`, `v17-vanilla-arm`, `v17-error-suite` |

## Scopes

| Agent | `paths_allowed` | `paths_denied` |
|-------|-----------------|----------------|
| `v17-loom-arm` | `tests/loom-eval/v17-loom/**`, `src/wordfreq/**`, `tests/wordfreq/**` | `tests/loom-eval/v17-vanilla/**`, `tests/loom-eval/v17-errors/**`, `tests/loom-eval/v17-report/**` |
| `v17-vanilla-arm` | `tests/loom-eval/v17-vanilla/**`, `src/wordfreq/**`, `tests/wordfreq/**` | `tests/loom-eval/v17-loom/**`, `tests/loom-eval/v17-errors/**`, `tests/loom-eval/v17-report/**` |
| `v17-error-suite` | `tests/loom-eval/v17-errors/**` | `tests/loom-eval/v17-loom/**`, `tests/loom-eval/v17-vanilla/**`, `tests/loom-eval/v17-report/**` |
| `v17-report` | `tests/loom-eval/v17-report/**` | `tests/loom-eval/v17-loom/**`, `tests/loom-eval/v17-vanilla/**`, `tests/loom-eval/v17-errors/**` |

No scope overlap across agents.

## Execution Flow

```
Step 1: Create 4 worktrees + branches
        git worktree add .worktrees/v17-loom-arm    -b loom/v17-loom-arm
        git worktree add .worktrees/v17-vanilla-arm  -b loom/v17-vanilla-arm
        git worktree add .worktrees/v17-error-suite  -b loom/v17-error-suite
        git worktree add .worktrees/v17-report       -b loom/v17-report

Step 2: Write TASK.md + AGENT.json for each. Commit.

Step 3: PLANNING PHASE -- spawn v17-loom-arm and v17-vanilla-arm in parallel
        (v17-error-suite also plans in parallel, but implements after both arms)
        All three write PLAN.md and return.

Step 4: PLAN GATE -- orchestrator reviews all 3 PLAN.md files
        - v17-loom-arm: must describe full LOOM ceremony for inner task
        - v17-vanilla-arm: must describe vanilla Agent spawning for same task
        - v17-error-suite: must describe how each error scenario will be injected
        Check for scope overlaps, approve or feedback.

Step 5: IMPLEMENTATION -- spawn v17-loom-arm and v17-vanilla-arm in parallel
        Both execute their respective arms of the experiment.

Step 6: INTEGRATE -- merge both arms (no deps between them). Validate.

Step 7: Spawn v17-error-suite (depends on both arms being integrated).
        It runs the four error scenarios.

Step 8: INTEGRATE v17-error-suite. Validate.

Step 9: Spawn v17-report (depends on all three).
        Reads all MEMORY.md files + measurement files. Writes comparison.

Step 10: INTEGRATE v17-report. Clean up all worktrees.
```

## LOOM Features Exercised

| Feature | How |
|---|---|
| Worktree isolation | 4 meta-agents, each in its own worktree |
| Parallel planning | 3 agents plan simultaneously |
| Plan gate | Orchestrator reviews 3 plans before implementation |
| Parallel implementation | 2 arms implement simultaneously |
| Sequential dependency | Error suite waits for both arms; report waits for all three |
| Commit trailers | Every agent commit has Agent-Id + Session-Id |
| STATUS.md lifecycle | PLANNING -> IMPLEMENTING -> COMPLETED |
| MEMORY.md handoff | Arms write findings; error suite and report read them |
| Scope enforcement | Verified at integration: arms cannot write to each other's directories |
| Worktree cleanup | All 4 removed at end |
| **Nested LOOM** | v17-loom-arm itself runs LOOM internally (LOOM orchestrating LOOM) |

## Features NOT Tested

- BLOCKED/FAILED states at the meta level (only tested within the error suite scenarios)
- Heartbeat enforcement (would require long-running agents)
- Continuation agents at the meta level

## Expected Findings (Hypotheses)

These are predictions to be validated or refuted by the experiment:

1. **Overhead**: LOOM will use ~2x more commits and ~1.5x more orchestrator actions than vanilla, due to protocol file commits and the two-phase spawn.
2. **Correctness**: Both arms will produce functionally equivalent code. LOOM's plan gate may catch a design mismatch (e.g., wrong function signature) before implementation; vanilla will only catch it at test time.
3. **Auditability**: LOOM will score significantly higher (estimated 8-10 vs. 2-4 out of 10) because every decision, state change, and scope boundary is recorded in structured, parseable files.
4. **Error recovery**: LOOM will detect and recover from all four error scenarios. Vanilla will fail to detect scope violations and will lack structured checkpoints on resource exhaustion.
5. **Net assessment**: LOOM's overhead is justified for multi-agent tasks where auditability and error handling matter. For single-agent, low-risk tasks, vanilla is sufficient.
