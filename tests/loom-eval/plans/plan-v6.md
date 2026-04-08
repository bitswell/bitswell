# LOOM Evaluation Plan v6 -- Max Parallelism

## Goal

Stress-test LOOM's ability to manage maximum concurrency: 10 fully independent agents running in parallel with zero inter-agent dependencies. Every agent writes to its own isolated directory, so integration can happen in any order. This exercises scope isolation at scale, the plan gate with 10 simultaneous reviews, and proves that LOOM's integration step is order-independent when no dependency edges exist.

## Variant: Max Parallelism

- **10 agents**, each with a distinct, small, self-contained task.
- **Zero dependencies** -- every agent's `dependencies` array is empty.
- All planning spawns happen in a single message (10 parallel Agent calls).
- All implementation spawns happen in a single message (10 parallel Agent calls).
- Integration order is deliberately randomized (not alphabetical, not creation order) to prove order-independence.
- Approaches the protocol's default `max_agents: 10` ceiling (Section 7.3).

## Agent Decomposition

| # | Agent ID | Task | Scope | Dependencies |
|---|----------|------|-------|-------------|
| 1 | `gen-uuid-lib` | Write a tiny UUID v4 generator function and unit test | `tests/loom-eval/gen-uuid-lib/**` | none |
| 2 | `ascii-banner` | Generate an ASCII art banner renderer that takes a string and returns block-letter output | `tests/loom-eval/ascii-banner/**` | none |
| 3 | `rot13-codec` | Implement ROT13 encode/decode with round-trip test | `tests/loom-eval/rot13-codec/**` | none |
| 4 | `word-counter` | Write a word-frequency counter that reads a text string and returns sorted counts | `tests/loom-eval/word-counter/**` | none |
| 5 | `slug-maker` | Create a URL slug generator (lowercases, replaces spaces/special chars) with edge-case tests | `tests/loom-eval/slug-maker/**` | none |
| 6 | `temp-convert` | Implement Celsius/Fahrenheit/Kelvin converter with property-based test cases | `tests/loom-eval/temp-convert/**` | none |
| 7 | `morse-codec` | Build a Morse code encoder/decoder with a lookup table and tests | `tests/loom-eval/morse-codec/**` | none |
| 8 | `palindrome-check` | Write a palindrome checker that handles Unicode normalization, with tests | `tests/loom-eval/palindrome-check/**` | none |
| 9 | `base64-codec` | Implement Base64 encode/decode (no stdlib) with conformance tests | `tests/loom-eval/base64-codec/**` | none |
| 10 | `roman-numeral` | Write a Roman numeral to/from integer converter with boundary tests | `tests/loom-eval/roman-numeral/**` | none |

Each task is intentionally small (one function + tests) to keep agent token usage low and minimize the chance of BLOCKED/FAILED states, which are not the focus of this evaluation variant.

## Scopes

| Agent | `paths_allowed` | `paths_denied` |
|-------|-----------------|----------------|
| `gen-uuid-lib` | `tests/loom-eval/gen-uuid-lib/**` | `[]` |
| `ascii-banner` | `tests/loom-eval/ascii-banner/**` | `[]` |
| `rot13-codec` | `tests/loom-eval/rot13-codec/**` | `[]` |
| `word-counter` | `tests/loom-eval/word-counter/**` | `[]` |
| `slug-maker` | `tests/loom-eval/slug-maker/**` | `[]` |
| `temp-convert` | `tests/loom-eval/temp-convert/**` | `[]` |
| `morse-codec` | `tests/loom-eval/morse-codec/**` | `[]` |
| `palindrome-check` | `tests/loom-eval/palindrome-check/**` | `[]` |
| `base64-codec` | `tests/loom-eval/base64-codec/**` | `[]` |
| `roman-numeral` | `tests/loom-eval/roman-numeral/**` | `[]` |

All 10 agents have strictly non-overlapping scopes. No two agents can touch the same file.

## Execution Flow

```
Step 1: Create 10 worktrees + branches (sequential, fast)
        git worktree add .worktrees/gen-uuid-lib      -b loom/gen-uuid-lib
        git worktree add .worktrees/ascii-banner       -b loom/ascii-banner
        git worktree add .worktrees/rot13-codec        -b loom/rot13-codec
        git worktree add .worktrees/word-counter       -b loom/word-counter
        git worktree add .worktrees/slug-maker         -b loom/slug-maker
        git worktree add .worktrees/temp-convert       -b loom/temp-convert
        git worktree add .worktrees/morse-codec        -b loom/morse-codec
        git worktree add .worktrees/palindrome-check   -b loom/palindrome-check
        git worktree add .worktrees/base64-codec       -b loom/base64-codec
        git worktree add .worktrees/roman-numeral      -b loom/roman-numeral

Step 2: Write TASK.md + AGENT.json into each worktree. Commit each.
        All 10 AGENT.json files have "dependencies": [].

Step 3: PLANNING PHASE -- spawn all 10 agents in a single message
        (10 parallel Agent calls, one per worktree)
        Each agent reads TASK.md + AGENT.json, writes PLAN.md + STATUS.md, commits, returns.

Step 4: PLAN GATE -- orchestrator reads all 10 PLAN.md files
        Checks:
          a. Scope compliance: each plan's "Files to Modify" stays within its allowed paths.
          b. No scope overlap: no two plans touch the same file (should be impossible
             given disjoint scopes, but verify).
          c. Reasonable approach: each plan is a sensible solution for its micro-task.
          d. No agent declares dependencies it shouldn't have.
        Approve all 10 or provide feedback to specific agents and re-plan.

Step 5: IMPLEMENTATION PHASE -- re-spawn all 10 agents in a single message
        (10 parallel Agent calls)
        Each agent reads PLAN.md, implements, writes MEMORY.md, sets STATUS.md
        to COMPLETED, commits, returns.

Step 6: INTEGRATION -- merge all 10 in deliberately shuffled order
        Use a non-obvious order to prove order-independence:
          1. morse-codec        (agent #7)
          2. slug-maker         (agent #5)
          3. base64-codec       (agent #9)
          4. gen-uuid-lib       (agent #1)
          5. palindrome-check   (agent #8)
          6. temp-convert       (agent #6)
          7. word-counter       (agent #4)
          8. roman-numeral      (agent #10)
          9. rot13-codec        (agent #3)
         10. ascii-banner       (agent #2)

        For each integration:
          a. Verify STATUS.md shows COMPLETED.
          b. Verify all changed files are within the agent's scope.
          c. git merge --no-ff loom/<agent-id>
          d. Run project validation (if any) after each merge.

Step 7: Read MEMORY.md from all 10 agents. Compile any cross-cutting findings.

Step 8: Clean up all 10 worktrees.
        git worktree remove .worktrees/<agent-id>   (x10)
```

## What This Plan Tests

### Primary targets (concurrency and isolation at scale)

| Feature | How exercised |
|---------|---------------|
| Max concurrent agents | 10 agents, matching the protocol's default limit (Section 7.3) |
| Fully parallel planning | 10 Agent tool calls in a single message during Step 3 |
| Fully parallel implementation | 10 Agent tool calls in a single message during Step 5 |
| Plan gate at scale | Orchestrator reviews 10 plans simultaneously for overlaps |
| Scope isolation (10-way) | 10 disjoint scopes, verified at plan gate and integration |
| Order-independent integration | Deliberately shuffled merge order proves no hidden ordering assumptions |
| Worktree isolation (10-way) | 10 simultaneous worktrees, no cross-contamination |

### Secondary targets (standard LOOM features still exercised)

| Feature | How exercised |
|---------|---------------|
| Commit trailers | 10 agents, each producing multiple commits with Agent-Id + Session-Id |
| STATUS.md lifecycle | All 10 agents traverse PLANNING -> IMPLEMENTING -> COMPLETED |
| MEMORY.md handoff | All 10 agents write MEMORY.md; orchestrator reads all at Step 7 |
| Scope enforcement | Verified at integration for every agent |
| Worktree cleanup | All 10 worktrees removed at Step 8 |
| AGENT.json validation | 10 distinct AGENT.json files with empty dependency arrays |
| Branch naming convention | 10 branches following `loom/<agent-id>` pattern |

### Stress characteristics

| Dimension | Value |
|-----------|-------|
| Total agents | 10 |
| Max concurrent agents (planning) | 10 |
| Max concurrent agents (implementing) | 10 |
| Dependency edges | 0 |
| Scope overlaps | 0 |
| Integration orderings possible | 10! = 3,628,800 |
| Total worktrees active simultaneously | 10 |

## Features NOT Tested

- **Dependency DAG** -- deliberately excluded. All agents are independent. See plan-v0 for dependency testing.
- **BLOCKED/FAILED states** -- tasks are kept trivially small to minimize failure likelihood.
- **Resource limit recovery / continuation agents** -- small tasks should complete well within budget.
- **Merge conflict recovery** -- disjoint scopes make conflicts impossible.
- **Heartbeat enforcement** -- tasks are too short to trigger the 5-minute heartbeat window.
- **Cross-agent MEMORY.md reads** -- no agent needs to read another's findings since there are no dependencies.
- **Tiered/phased implementation** -- there is only one implementation tier since there are no dependencies.

## Success Criteria

1. All 10 agents reach COMPLETED status without BLOCKED or FAILED.
2. All 10 agents' commits contain valid Agent-Id and Session-Id trailers.
3. All 10 integrations succeed without merge conflicts.
4. No agent modifies files outside its declared scope.
5. Integration in the specified shuffled order produces the same final workspace state as would any other order (verified by checking that no merge depends on a prior merge's content).
6. All 10 worktrees are cleanly removed at the end.
7. The orchestrator never exceeds the max_agents=10 limit at any point.
