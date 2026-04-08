# LOOM Evaluation Plan v0 (Original)

## Goal

Evaluate and test the LOOM skill at `.claude/skills/loom/` by using LOOM itself to orchestrate the evaluation.

## Agent Decomposition

| Agent ID | Role | Task | Dependencies |
|----------|------|------|-------------|
| `schema-audit` | Analyst | Cross-reference all file format definitions across the 5 skill documents. Flag mismatches between schemas.md, protocol.md, worker-template.md, and examples.md. | none |
| `command-audit` | Analyst | Validate every git command pattern in SKILL.md and examples.md for syntactic correctness. Test each in a scratch repo. | none |
| `protocol-audit` | Analyst | Review state machine completeness, security model gaps, error recovery paths, dependency DAG handling. Identify underspecified or contradictory rules. | none |
| `eval-report` | Reporter | Read MEMORY.md from all 3 audit agents. Compile a single findings report with severity ratings and fix recommendations. | `schema-audit`, `command-audit`, `protocol-audit` |

## Scopes

| Agent | `paths_allowed` | `paths_denied` |
|-------|-----------------|----------------|
| `schema-audit` | `tests/loom-eval/schema-audit/**` | `[]` |
| `command-audit` | `tests/loom-eval/command-audit/**` | `[]` |
| `protocol-audit` | `tests/loom-eval/protocol-audit/**` | `[]` |
| `eval-report` | `tests/loom-eval/report/**` | `[]` |

No scope overlap -- all four agents write to isolated directories.

## Execution Flow

```
Step 1: Create 4 worktrees + branches
        git worktree add .worktrees/schema-audit   -b loom/schema-audit
        git worktree add .worktrees/command-audit   -b loom/command-audit
        git worktree add .worktrees/protocol-audit  -b loom/protocol-audit
        git worktree add .worktrees/eval-report     -b loom/eval-report

Step 2: Write TASK.md + AGENT.json into each worktree. Commit.

Step 3: PLANNING PHASE -- spawn 3 audit agents in parallel
        schema-audit, command-audit, protocol-audit all write PLAN.md

Step 4: PLAN GATE -- orchestrator reads all 3 PLAN.md files
        Check for scope overlaps, unrealistic approaches, missing coverage.
        Approve or send feedback.

Step 5: IMPLEMENTATION PHASE -- re-spawn 3 audit agents in parallel
        Each does analysis work and writes findings.

Step 6: INTEGRATE -- merge all 3 in any order (no deps between them)
        Validate after each merge.

Step 7: PLAN + IMPLEMENT eval-report (depends on all 3 being integrated)
        Reads 3 MEMORY.md files, compiles report.

Step 8: INTEGRATE eval-report. Clean up all worktrees.
```

## LOOM Features Exercised

| Feature | How |
|---|---|
| Worktree isolation | 4 agents, 4 worktrees, non-overlapping scopes |
| Parallel planning | 3 agents plan simultaneously |
| Plan gate | Orchestrator reviews 3 plans before any implementation |
| Parallel implementation | 3 agents implement simultaneously |
| Commit trailers | Every agent commit must have Agent-Id + Session-Id |
| STATUS.md lifecycle | PLANNING -> IMPLEMENTING -> COMPLETED |
| MEMORY.md handoff | Audit agents write findings; report agent reads them |
| Dependency ordering | eval-report waits for all 3 audits |
| Scope enforcement | Verified at integration time |
| Worktree cleanup | All 4 removed at end |

## Features NOT Tested

- BLOCKED/FAILED states
- Resource limit recovery / continuation agents
- Merge conflict recovery
- Heartbeat enforcement
