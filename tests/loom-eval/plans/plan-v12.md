# LOOM Evaluation Plan v12 — Real-World Task

## Thesis

Plan v0 evaluates LOOM by having agents audit the LOOM skill itself — a self-referential exercise that produces reports but no lasting value for the repo. This plan evaluates LOOM by having it deliver something the repo actually needs. The evaluation question changes from "did the protocol execute correctly?" to "did the protocol produce a useful artifact that stays in the codebase?"

## The Deliverable

A complete CI validation pipeline for the bitswell identity framework:

1. **`ci/validate.sh`** — A portable shell test runner that wraps the existing Rust test suite (`tests/src/main.rs`) and adds structural checks the Rust binary does not cover.
2. **`ci/lint-markdown.sh`** — A markdown linter for identity files: checks YAML front matter validity, detects broken internal links (`[connects-to: value-N]` references), enforces the required section structure in agent identity files, and flags empty or stub files.
3. **`.github/workflows/validate.yml`** — A GitHub Actions workflow that runs both scripts on push/PR, caches the Rust build, and reports results as check annotations.
4. **`tests/src/main.rs` fixes** — Any bugs or gaps found in the existing Rust validation suite during the process of integrating it into CI.

The repo currently has no CI. The Rust test suite exists but has never been wired into automated checks. If LOOM delivers a working pipeline that passes on the current repo state, the evaluation succeeds.

## Why This Tests LOOM Well

- The task is decomposable into independent agents with clear scopes.
- The agents produce files that can be tested (the CI pipeline either runs or it does not).
- Scope enforcement matters: agents writing CI config must not touch identity files, and vice versa.
- Dependencies are real: the workflow agent depends on both script agents.
- The plan gate matters: overlapping file modifications between the shell scripts and the workflow would cause merge conflicts.
- Success is binary and observable: `gh workflow run validate` either passes or it does not.

## Agent Decomposition

| Agent ID | Role | Task | Dependencies |
|----------|------|------|-------------|
| `ci-runner` | Builder | Create `ci/validate.sh` — a shell script that builds and runs the Rust test suite, captures output, reports pass/fail with exit codes. Must handle the case where `cargo` is not installed (skip gracefully). | none |
| `ci-linter` | Builder | Create `ci/lint-markdown.sh` — a shell script that validates markdown structure across `agents/`, `memory/`, and `questions/`. Checks: YAML front matter in files that use it, required sections in identity files, non-empty content, broken `[connects-to: value-N]` references, consistent heading levels. | none |
| `ci-workflow` | Builder | Create `.github/workflows/validate.yml` — GitHub Actions workflow triggered on push and pull_request. Installs Rust toolchain, runs `ci/validate.sh` and `ci/lint-markdown.sh`, caches `tests/target/`. Uses appropriate runner and permissions. | `ci-runner`, `ci-linter` |
| `ci-fixup` | Fixer | Run the full pipeline locally. Fix any failures in the Rust test suite or the shell scripts that surface when tested against the actual repo state. Write a summary of what broke and what was fixed. | `ci-runner`, `ci-linter`, `ci-workflow` |

## Scopes

| Agent | `paths_allowed` | `paths_denied` |
|-------|-----------------|----------------|
| `ci-runner` | `ci/validate.sh` | `[]` |
| `ci-linter` | `ci/lint-markdown.sh` | `[]` |
| `ci-workflow` | `.github/workflows/validate.yml`, `.github/workflows/**` | `[]` |
| `ci-fixup` | `ci/**`, `.github/workflows/**`, `tests/src/**` | `agents/**`, `memory/**`, `questions/**` |

No scope overlap between the three builder agents. `ci-fixup` has broader scope but only runs after the builders are integrated, and it is explicitly denied from touching identity content.

## Execution Flow

```
Step 1: Create 4 worktrees + branches
        git worktree add .worktrees/ci-runner   -b loom/ci-runner
        git worktree add .worktrees/ci-linter   -b loom/ci-linter
        git worktree add .worktrees/ci-workflow  -b loom/ci-workflow
        git worktree add .worktrees/ci-fixup     -b loom/ci-fixup

Step 2: Write TASK.md + AGENT.json into each worktree. Commit.

Step 3: PLANNING PHASE — spawn ci-runner and ci-linter in parallel
        (ci-workflow and ci-fixup also plan in parallel — planning
        does not require deps to be integrated)
        All 4 agents write PLAN.md and return.

Step 4: PLAN GATE — orchestrator reads all 4 PLAN.md files
        Check:
        - ci-runner's plan produces a script with a clear exit-code contract
        - ci-linter's plan covers the required checks (front matter, sections, links)
        - ci-workflow references the correct script paths and has caching
        - ci-fixup understands it must actually execute the pipeline, not just read it
        - No scope overlaps between builder agents
        Approve or provide feedback.

Step 5: IMPLEMENTATION PHASE, Tier 1 — spawn ci-runner and ci-linter in parallel
        Both are independent. They write their shell scripts, test them locally,
        commit, set COMPLETED.

Step 6: INTEGRATE Tier 1 — merge ci-runner and ci-linter in either order
        Validate after each merge:
        - ci/validate.sh exists, is executable, runs without syntax errors
        - ci/lint-markdown.sh exists, is executable, runs without syntax errors

Step 7: IMPLEMENTATION PHASE, Tier 2 — spawn ci-workflow
        ci-workflow depends on both scripts being integrated so it can
        reference their actual paths and behavior.

Step 8: INTEGRATE ci-workflow
        Validate:
        - .github/workflows/validate.yml is valid YAML
        - Workflow references ci/validate.sh and ci/lint-markdown.sh
        - actionlint passes (if available), or manual structure check

Step 9: IMPLEMENTATION PHASE, Tier 3 — spawn ci-fixup
        ci-fixup depends on all three prior agents being integrated.
        It runs the full pipeline against the current repo state and fixes
        whatever breaks.

Step 10: INTEGRATE ci-fixup
         Validate:
         - ci/validate.sh exits 0 on the current repo
         - ci/lint-markdown.sh exits 0 on the current repo
         - No regressions in tests/src/main.rs (cargo build succeeds)

Step 11: Read MEMORY.md from all 4 agents. Clean up worktrees.
```

## Dependency DAG

```
ci-runner ──┐
            ├──> ci-workflow ──> ci-fixup
ci-linter ──┘
```

Planning is fully parallel (4 agents). Implementation has three tiers:
- Tier 1: `ci-runner` + `ci-linter` (parallel)
- Tier 2: `ci-workflow` (serial, depends on Tier 1)
- Tier 3: `ci-fixup` (serial, depends on Tier 2)

## LOOM Features Exercised

| Feature | How |
|---|---|
| Worktree isolation | 4 agents, 4 worktrees, non-overlapping scopes |
| Parallel planning | All 4 agents plan simultaneously |
| Plan gate | Orchestrator reviews 4 plans before any implementation |
| Parallel implementation | Tier 1: 2 agents implement simultaneously |
| Dependency ordering | Tier 2 waits for Tier 1; Tier 3 waits for Tier 2 |
| Commit trailers | Every agent commit must have Agent-Id + Session-Id |
| STATUS.md lifecycle | PLANNING -> IMPLEMENTING -> COMPLETED |
| MEMORY.md handoff | Builder agents' findings inform ci-fixup's work |
| Scope enforcement | ci-fixup explicitly denied from touching identity files |
| Worktree cleanup | All 4 removed at end |
| Real validation at integration | Scripts are actually executed, not just checked for existence |

## Success Criteria

The evaluation passes if and only if ALL of the following are true after LOOM completes:

1. **Files delivered.** `ci/validate.sh`, `ci/lint-markdown.sh`, and `.github/workflows/validate.yml` exist in the workspace.
2. **Scripts are executable.** Both shell scripts have `+x` permissions and run without syntax errors (`bash -n` passes).
3. **validate.sh works.** Running `ci/validate.sh` from the repo root builds the Rust test suite and reports results. Exit code 0 on a healthy repo.
4. **lint-markdown.sh works.** Running `ci/lint-markdown.sh` from the repo root checks identity file structure and reports results. Exit code 0 on the current repo state.
5. **Workflow is valid.** `.github/workflows/validate.yml` is valid YAML and references both scripts.
6. **No identity file modifications.** LOOM agents did not modify any files under `agents/`, `memory/`, or `questions/`. The deliverable is infrastructure, not content.
7. **Protocol compliance.** All agent commits have proper trailers, STATUS.md lifecycle was followed, scope was respected.

## Features NOT Tested

- BLOCKED/FAILED states (unless ci-fixup genuinely hits a wall)
- Resource limit recovery / continuation agents
- Merge conflict recovery (scopes are designed to avoid this)
- Heartbeat enforcement (short-lived agents)

## What Makes This Different from v0

| Aspect | v0 (Audit) | v12 (Real-World Task) |
|--------|-----------|----------------------|
| **Output** | Reports about the LOOM skill | Working CI pipeline |
| **Lasting value** | Findings may become stale | Pipeline stays in the repo |
| **Testability** | Reports require human judgment | Scripts either run or they do not |
| **Risk** | Low — agents only write reports | Medium — agents write executable code that must work |
| **Scope enforcement** | Easy — all outputs in isolated test dirs | Meaningful — agents must not touch identity files |
| **Dependencies** | One dependent agent | Three-tier dependency chain |
| **Integration validation** | Existence checks | Execution checks (scripts are run) |

The fundamental difference: v0 asks "can LOOM orchestrate agents?" while v12 asks "can LOOM deliver real work?"
