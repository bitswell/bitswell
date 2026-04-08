# LOOM Evaluation Plan v20 -- Cross-Repo Portability

## Goal

Determine whether the LOOM skill is portable across arbitrary git repositories. Identify every hardcoded assumption, bitswell-specific reference, implicit prerequisite, and structural coupling that would prevent LOOM from operating in a foreign repo. Test portability by mentally applying LOOM to three target environments: (1) a fresh empty repo, (2) a large monorepo, and (3) a repo with git submodules.

## Evaluation Strategy

Instead of spawning agents to audit documents (as in plan-v0), this plan spawns agents that each simulate LOOM execution in a different target environment, cataloging every failure point. A fourth agent cross-references findings into a portability scorecard.

---

## Agent Decomposition

| Agent ID | Role | Task | Dependencies |
|----------|------|------|-------------|
| `env-empty` | Tester | Simulate running LOOM in a fresh `git init` repo with zero files, zero history. Walk through every protocol step and record what fails or assumes pre-existing content. | none |
| `env-monorepo` | Tester | Simulate LOOM in a monorepo with 50+ packages, nested workspaces, CI that gates on per-package test suites, and path-scoped CODEOWNERS. Identify scaling and scoping failures. | none |
| `env-submodules` | Tester | Simulate LOOM in a repo that uses git submodules for shared libraries. Identify worktree/submodule interaction failures and scope violations that cross submodule boundaries. | none |
| `portability-report` | Reporter | Read MEMORY.md from all 3 environment agents. Produce a portability scorecard, severity-ranked issue list, and recommendations for making LOOM environment-agnostic. | `env-empty`, `env-monorepo`, `env-submodules` |

## Scopes

| Agent | `paths_allowed` | `paths_denied` |
|-------|-----------------|----------------|
| `env-empty` | `tests/loom-eval/env-empty/**` | `[]` |
| `env-monorepo` | `tests/loom-eval/env-monorepo/**` | `[]` |
| `env-submodules` | `tests/loom-eval/env-submodules/**` | `[]` |
| `portability-report` | `tests/loom-eval/portability-report/**` | `[]` |

No scope overlap.

---

## What Each Agent Investigates

### env-empty: Fresh Empty Repo

Walk through the full LOOM 10-step core flow assuming the repo was just created with `git init` and contains nothing.

**Specific checks:**

1. **No HEAD commit.** `git worktree add .worktrees/<id> -b loom/<id>` requires a commit to branch from. A repo with zero commits has no HEAD. Does LOOM document this prerequisite? (Answer: no -- SKILL.md step 3 assumes HEAD exists.)

2. **No validation command.** SKILL.md step 9 says "run project validation after each merge." Protocol.md section 5.3 step 5 says "Run project validation (tests, linting -- project-defined, not protocol-defined)." What happens when there is no test runner, no linter, no CI? Is the step skipped? Is it an error? The protocol does not specify behavior when validation is undefined.

3. **No .claude/ directory.** The skill is installed at `.claude/skills/loom/`. If LOOM is used in a repo that does not have this directory, does the worker template reference any local paths that would break? (Check: worker-template.md uses `{{WORKTREE_PATH}}` which is absolute -- this is fine. But the orchestrator must be invoked with the skill loaded, which means the skill files must exist somewhere.)

4. **No existing branch structure.** The branch naming convention `loom/<agent-id>` is clean, but does LOOM check for pre-existing `loom/` branches from a prior run? What if stale branches exist? The protocol says failed branches are retained for 30 days -- how does a fresh run disambiguate?

5. **Empty scope.** In a repo with no files, `paths_allowed` globs match nothing. Can an agent create new files under an allowed glob, or does the scope only permit modification of existing files? The schemas.md says "paths the agent is permitted to modify" -- does "modify" include "create"?

6. **No `uuidgen` or `python3`.** SKILL.md step 4 of the worker injection pattern says to generate a UUID with `uuidgen` or `python3`. Neither is guaranteed to exist. What if the environment is a minimal Docker container?

### env-monorepo: Large Monorepo

Simulate a monorepo with structure like:
```
packages/
  auth/
  api/
  shared/
  web/
apps/
  mobile/
  desktop/
.github/
  CODEOWNERS
```

**Specific checks:**

1. **Worktree disk usage.** Each worktree is a full checkout. Protocol section 7.3 sets a 5 GB limit per worktree, but a monorepo might be 10+ GB. With 10 concurrent agents, that is 100+ GB of worktrees. Does LOOM address monorepo-scale disk costs? (Answer: no.)

2. **Scope glob depth.** AGENT.json scope uses globs relative to repo root. In a monorepo, scopes like `packages/auth/**` work fine, but what about cross-package changes? If a task requires modifying `packages/auth/` AND `packages/shared/`, the agent needs both in `paths_allowed`. The protocol allows this, but the plan gate must detect when two agents both need `packages/shared/` -- the skill says to "check for scope overlaps" but gives no algorithm for resolving them.

3. **Validation per package.** The protocol says "run project validation" but monorepos often have per-package test suites. After integrating an agent that modified `packages/auth/`, do you run `npm test` at root (slow, runs everything) or `npm test --workspace=auth` (fast, targeted)? The protocol is silent on this.

4. **CODEOWNERS conflicts.** If the monorepo uses CODEOWNERS and a LOOM agent creates a PR-like integration, CODEOWNERS review requirements are bypassed. This is a governance gap, not a protocol gap, but it matters for real adoption.

5. **Concurrent worktree limits.** Git worktrees share a single `.git` directory. With many agents doing concurrent `git add`/`git commit` operations, is there lock contention on the git index? Git uses `.git/worktrees/<name>/index` per worktree, so this should be safe -- but the protocol does not discuss git-level concurrency at all.

6. **Branch namespace pollution.** 10 agents per task, multiple tasks per day, 30-day branch retention. In a busy monorepo, the `loom/` namespace accumulates hundreds of branches. The protocol specifies no garbage collection mechanism beyond the 30-day retention note.

### env-submodules: Repo with Submodules

Simulate a repo structured like:
```
src/
lib/shared/  (submodule -> git@github.com:org/shared-lib.git)
lib/proto/   (submodule -> git@github.com:org/proto-defs.git)
```

**Specific checks:**

1. **Worktree + submodule interaction.** `git worktree add` does NOT automatically initialize submodules in the new worktree. The agent's worktree will have empty directories where submodules should be. The protocol never mentions `git submodule update --init`. This is a hard break -- agent code that imports from `lib/shared/` will fail.

2. **Scope across submodule boundaries.** If `paths_allowed` includes `lib/shared/**`, does the agent modify files in the submodule? Those files belong to a different repository. Committing changes inside a submodule changes the submodule's HEAD, which then requires updating the parent repo's submodule reference. The LOOM protocol's commit/integrate model does not account for this two-repo commit dance.

3. **Branch naming in submodules.** If an agent needs to modify submodule content, it would need a `loom/<id>` branch in the submodule repo too. The protocol assumes one repo, one branch namespace.

4. **Merge across submodule changes.** `git merge --no-ff loom/<id>` in the parent repo will see the submodule pointer changed. If two agents both updated the same submodule, this creates a submodule conflict that standard `git merge` does not resolve cleanly. The conflict recovery path in LOOM (abort + rebase or fresh agent) works, but the root cause (submodule pointer divergence) is never diagnosed.

5. **Shallow clones.** Many CI environments use `--depth=1` clones. Worktrees created from shallow repos have limited history. The `base_commit` field in STATUS.md may reference a commit not present in the shallow history. The protocol assumes full history.

---

## Hardcoded Assumptions Catalog

These are assumptions baked into the LOOM skill files that limit portability. Each agent should verify and expand this list during their environment simulation.

| # | Assumption | Where | Impact |
|---|-----------|-------|--------|
| H1 | Repo has at least one commit (HEAD exists) | SKILL.md step 3, examples.md all examples | Breaks on fresh `git init` |
| H2 | `uuidgen` or `python3` available on system | SKILL.md worker injection step 4 | Breaks on minimal environments |
| H3 | `/home/user/project` as example path | examples.md line 3 | Cosmetic, but suggests Unix-only thinking. No Windows path examples. |
| H4 | `npm test` as validation command | examples.md examples 1-3 | Hardcodes Node.js ecosystem. Rust, Go, Python repos use different commands. |
| H5 | `.worktrees/` directory is gitignored | Implicit -- never stated | If not gitignored, worktree contents appear as untracked files in parent. Protocol never says to add `.worktrees/` to `.gitignore`. |
| H6 | No submodules | Entire protocol | Worktrees created without `--recurse-submodules` (not even a git-native flag for `git worktree add`) |
| H7 | Flat repository (not a monorepo) | Scope model, validation model | No per-package validation, no workspace-aware scoping |
| H8 | Full git history available | STATUS.md `base_commit`, merge operations | Breaks on shallow clones |
| H9 | Single remote | Implicit | Protocol never addresses multi-remote setups or fork workflows |
| H10 | Unix-like OS | All bash commands, path separators | No Windows compatibility consideration |

---

## Execution Flow

```
Step 1: Create 4 worktrees + branches
        git worktree add .worktrees/env-empty       -b loom/env-empty
        git worktree add .worktrees/env-monorepo    -b loom/env-monorepo
        git worktree add .worktrees/env-submodules  -b loom/env-submodules
        git worktree add .worktrees/portability-report -b loom/portability-report

Step 2: Write TASK.md + AGENT.json into each worktree. Commit.
        Each TASK.md contains the relevant section from "What Each Agent
        Investigates" above, plus the Hardcoded Assumptions Catalog as
        a starting checklist.

Step 3: PLANNING PHASE -- spawn 3 environment agents in parallel
        env-empty, env-monorepo, env-submodules all write PLAN.md

Step 4: PLAN GATE -- orchestrator reads all 3 PLAN.md files
        Verify: no scope overlaps, each agent covers its assigned checks,
        no agent is planning to actually create a real external repo
        (all simulation should be analysis + documentation, not execution).

Step 5: IMPLEMENTATION PHASE -- re-spawn 3 environment agents in parallel
        Each agent:
        - Walks through every LOOM protocol step in their target environment
        - For each step, records: works / breaks / ambiguous
        - Catalogs every assumption violation as a numbered finding
        - Writes findings to their scoped output directory
        - Writes MEMORY.md with structured findings

Step 6: INTEGRATE -- merge all 3 in any order (no deps between them)
        Validate after each merge (validation = presence of findings files).

Step 7: PLAN + IMPLEMENT portability-report (depends on all 3 integrated)
        Reads 3 MEMORY.md files. Produces:
        - Portability scorecard (0-10 per environment)
        - Severity-ranked issue list (critical / high / medium / low)
        - Recommendations for making LOOM portable
        - Proposed changes to SKILL.md, protocol.md, schemas.md

Step 8: INTEGRATE portability-report. Clean up all worktrees.
```

---

## Expected Findings (Hypotheses)

These are predictions about what the evaluation will discover. The agents should confirm or refute each.

### Critical (would prevent LOOM from functioning)

- **C1:** Fresh repo with no commits cannot create worktrees. LOOM provides no bootstrap step.
- **C2:** Submodule content is missing in agent worktrees. Any task touching submodule code fails silently (empty directories) or loudly (import errors).
- **C3:** Shallow clones may prevent worktree creation or cause merge failures when `base_commit` is unreachable.

### High (would cause incorrect behavior)

- **H1:** `.worktrees/` not gitignored causes noise in `git status`, may be accidentally committed.
- **H2:** Monorepo disk usage can exceed practical limits with multiple agents.
- **H3:** Scope model cannot express "files in this package AND files in the shared package" without overlapping with another agent's scope.

### Medium (would require workarounds)

- **M1:** No validation command specified for non-Node.js projects. Orchestrator must improvise.
- **M2:** Branch namespace pollution in long-running projects with no GC mechanism.
- **M3:** `uuidgen`/`python3` dependency is unnecessary -- the orchestrator (Claude Code) can generate UUIDs without shelling out.

### Low (cosmetic or documentation gaps)

- **L1:** All examples use Unix paths and `npm test`. No cross-platform examples.
- **L2:** No mention of `.gitignore` requirements for the `.worktrees/` directory.
- **L3:** Protocol does not discuss Windows path handling or PowerShell equivalents.

---

## LOOM Features Exercised

| Feature | How |
|---|---|
| Worktree isolation | 4 agents, 4 worktrees, non-overlapping scopes |
| Parallel planning | 3 agents plan simultaneously |
| Plan gate | Orchestrator reviews 3 plans before any implementation |
| Parallel implementation | 3 agents implement simultaneously |
| Commit trailers | Every agent commit must have Agent-Id + Session-Id |
| STATUS.md lifecycle | PLANNING -> IMPLEMENTING -> COMPLETED |
| MEMORY.md handoff | Environment agents write findings; report agent reads them |
| Dependency ordering | portability-report waits for all 3 environment agents |
| Scope enforcement | Verified at integration time |
| Worktree cleanup | All 4 removed at end |

## Features NOT Tested

- BLOCKED/FAILED states (no agent is expected to fail)
- Resource limit recovery / continuation agents
- Merge conflict recovery
- Heartbeat enforcement
- Cross-agent MEMORY.md reads during execution (only post-integration reads)

---

## Success Criteria

The evaluation succeeds if:

1. All 3 environment agents produce structured findings with clear works/breaks/ambiguous verdicts for every protocol step.
2. The portability report produces a scorecard with numeric ratings and actionable recommendations.
3. At least 5 previously undocumented portability issues are identified (beyond the 10 in the Hardcoded Assumptions Catalog).
4. Each critical finding includes a proposed fix (not just a problem statement).

## Difference from plan-v0

Plan-v0 evaluates LOOM's internal consistency (schema mismatches, command correctness, protocol gaps). This plan evaluates LOOM's external portability -- whether it works outside the environment it was designed in. The two plans are complementary: v0 asks "is LOOM correct?" while v20 asks "is LOOM general?"
