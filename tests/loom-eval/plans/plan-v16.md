# LOOM Evaluation Plan v16 -- Documentation Gap Analysis

## Goal

Evaluate the LOOM skill docs from the perspective of a first-time user who has never seen the protocol. Instead of testing protocol correctness, test **developer experience**: find every question the docs don't answer, every assumption they don't state, every ambiguity that would stall someone mid-implementation, and every error scenario where the docs leave the user guessing.

The evaluation itself is orchestrated via LOOM, exercising the protocol while auditing it.

## Rationale

Protocol specs tend to be written by authors who already hold the mental model. The result is documentation that is technically complete but practically opaque. A first-time user hits friction at predictable points: "What value do I put here?", "What happens if I skip this?", "This command failed -- now what?". This plan systematically catalogs those friction points.

## Agent Decomposition

| Agent ID | Role | Task | Dependencies |
|----------|------|------|-------------|
| `onboard-walkthrough` | Analyst | Simulate a cold-start: attempt to follow the docs from zero to a working single-agent LOOM run. Record every point of confusion, missing prerequisite, and unstated assumption. | none |
| `error-message-audit` | Analyst | Catalog every failure mode described in the docs (FAILED states, merge conflicts, scope violations, budget exhaustion, heartbeat timeout, etc). For each: does the doc say what the user will actually SEE? What error text? What git output? What should they do first? | none |
| `ambiguity-finder` | Analyst | Read all 5 doc files for language that is vague, contradictory, or open to multiple interpretations. Flag every SHOULD without a concrete default, every "may" that a worker agent would need to resolve at runtime, and every cross-file inconsistency. | none |
| `missing-faq` | Analyst | Generate the 30 most likely questions a first-time LOOM user would ask that are NOT answered anywhere in the current docs. Categorize by severity (blocks progress vs. causes confusion vs. nice-to-know). | none |
| `dx-report` | Reporter | Read MEMORY.md from all 4 analysts. Produce a Developer Experience Report: prioritized list of doc gaps, grouped by category, with severity and recommended fixes. | `onboard-walkthrough`, `error-message-audit`, `ambiguity-finder`, `missing-faq` |

## Scopes

| Agent | `paths_allowed` | `paths_denied` |
|-------|-----------------|----------------|
| `onboard-walkthrough` | `tests/loom-eval/onboard-walkthrough/**` | `[]` |
| `error-message-audit` | `tests/loom-eval/error-message-audit/**` | `[]` |
| `ambiguity-finder` | `tests/loom-eval/ambiguity-finder/**` | `[]` |
| `missing-faq` | `tests/loom-eval/missing-faq/**` | `[]` |
| `dx-report` | `tests/loom-eval/dx-report/**` | `[]` |

No scope overlap. Each agent writes to its own directory.

## Detailed Agent Tasks

### onboard-walkthrough

Act as a developer who just discovered LOOM and wants to use it. Starting from SKILL.md alone:

1. **Prerequisite check.** What do I need installed? What git version? Does this work on Windows? What about shallow clones? What about repos with submodules? The docs don't say.
2. **First run.** Follow the "Single Agent" recipe step by step. At each step, note:
   - Is the command copy-pasteable or does it have placeholders I need to fill in?
   - What working directory am I supposed to be in?
   - What happens if the command fails? (e.g., worktree already exists, branch already exists)
   - What should I see on success?
3. **Concept mapping.** Can I tell the difference between "workspace" and "worktree" from the docs alone? Between "agent" and "worker"? Between TASK.md scope fields and AGENT.json scope fields?
4. **Missing getting-started.** Is there a minimal runnable example? Or do I have to mentally assemble one from five different files?

Deliverable: A friction log -- ordered list of every stumbling block encountered, with the exact doc location (file + section) that should have helped but didn't.

### error-message-audit

For every failure mode the protocol defines:

1. **Catalog the failure.** List each way things can go wrong: scope violation at integration, missing commit trailers, invalid STATUS.md YAML, merge conflict, heartbeat timeout, budget exhaustion, cycle in dependency DAG, missing MEMORY.md, exit code 2 (catastrophic).
2. **Trace the user experience.** For each failure:
   - What git command or orchestrator action triggers the failure?
   - What will the user literally see in their terminal? (Error text, git output, exit code)
   - Does the doc tell them what to do next, or does it just say "MUST reject" without saying how?
3. **Ghost errors.** Identify failures that can happen but are never mentioned in the docs. Examples: what if `uuidgen` is not installed? What if the agent's branch is ahead of workspace HEAD in a way that makes `base_commit` validation ambiguous? What if STATUS.md has valid YAML but the `status` value has a typo like `COMPLEETED`?
4. **Recovery gaps.** For each error with a documented recovery path, assess: is the recovery actually actionable, or does it assume knowledge the doc hasn't provided?

Deliverable: An error experience matrix -- rows are failure modes, columns are {trigger, what user sees, what doc says to do, what's actually missing}.

### ambiguity-finder

Systematic scan of all five doc files for:

1. **Vague normative language.** Every SHOULD without a stated default or fallback. Every MAY where the consequences of choosing either way are unclear. Every "project-defined" delegation that leaves the user with no starting point.
2. **Cross-file contradictions.** Do the schemas in schemas.md match the examples in examples.md exactly? Does the worker-template.md reference fields that protocol.md defines differently? Does SKILL.md's "Command Patterns" section match examples.md's commands?
3. **Undefined terms.** Words used without definition: "Level 1" vs "Level 2" (what's the difference beyond budget tracking?), "workspace HEAD" (is this always main?), "project validation" (what if I have no tests?), "entry" (Section 2 defines it but it barely appears again).
4. **Numeric values without rationale.** Why 5 minutes for heartbeat? Why 10 max agents? Why 30 days retention? Are these configurable? How? Where is the config file?
5. **Format ambiguities.** STATUS.md says YAML front matter -- are the string values quoted or unquoted? Both appear in examples. Is `files_changed` 0 during PLANNING or omitted? The schema says "required when COMPLETED" but examples during PLANNING omit it entirely while the protocol.md example includes `files_changed: 0`.

Deliverable: Ambiguity register -- each entry has {location, quoted text, what's ambiguous, potential interpretations, recommended clarification}.

### missing-faq

Generate the 30 most likely questions from a first-time user, organized by category. Focus on questions where the answer is NOT in the docs:

1. **Setup questions.** How do I install LOOM? Is it a CLI? A library? A skill file I drop in? What's the minimum viable setup? Can I use it outside Claude Code?
2. **Operational questions.** Can I run LOOM on a CI server? What happens to worktrees if my machine crashes mid-run? Can I resume? How do I see the status of all agents at once? Is there a dashboard?
3. **Customization questions.** Can I change the heartbeat interval? Can I use a different branch naming convention? Can I add custom fields to STATUS.md? Can I use LOOM with a monorepo?
4. **Escape hatch questions.** How do I abort everything and clean up? How do I manually fix a stuck agent? Can I edit an agent's TASK.md while it's running? What if I want to cancel one agent but keep the others?
5. **Scale questions.** What happens with 50 agents? 100? Does the orchestrator hit its own context limit? How do I decompose a task that's too big for a single orchestrator context window?
6. **Integration questions.** Does LOOM work with GitHub Actions? With pre-commit hooks? With branch protection rules? With signed commits?

For each question, rate severity:
- **P0 (blocks progress):** User cannot continue without this answer.
- **P1 (causes confusion):** User can work around it but wastes significant time.
- **P2 (nice to know):** User wonders but isn't blocked.

Deliverable: Prioritized FAQ document with 30 questions, their severity, and where in the docs the answer should live.

### dx-report

Depends on all four analysts. Reads their MEMORY.md files and produces:

1. **Executive summary.** One paragraph: overall developer experience assessment.
2. **Top 10 gaps by severity.** The ten documentation problems most likely to cause a first-time user to give up or waste significant time. Each with: description, evidence from the analyst agents, recommended fix.
3. **Category breakdown.** Group all findings into:
   - Missing content (things the docs should say but don't)
   - Ambiguous content (things the docs say unclearly)
   - Contradictory content (things the docs say differently in different places)
   - Missing examples (concepts that need a concrete illustration)
   - Error experience (failure modes with poor or missing guidance)
4. **Recommended doc changes.** For each finding, a specific recommendation: which file, which section, what to add or change.
5. **Meta-observation.** How well did LOOM itself work for running this evaluation? Any friction from using the protocol to evaluate the protocol?

Deliverable: Single markdown report at `tests/loom-eval/dx-report/DX-REPORT.md`.

## Execution Flow

```
Step 1: Create 5 worktrees + branches
        git worktree add .worktrees/onboard-walkthrough  -b loom/onboard-walkthrough
        git worktree add .worktrees/error-message-audit   -b loom/error-message-audit
        git worktree add .worktrees/ambiguity-finder      -b loom/ambiguity-finder
        git worktree add .worktrees/missing-faq           -b loom/missing-faq
        git worktree add .worktrees/dx-report             -b loom/dx-report

Step 2: Write TASK.md + AGENT.json into each worktree. Commit.

Step 3: PLANNING PHASE -- spawn 4 analyst agents in parallel
        onboard-walkthrough, error-message-audit, ambiguity-finder, missing-faq
        all write PLAN.md and return.

Step 4: PLAN GATE -- orchestrator reads all 4 PLAN.md files
        Check for:
        - Scope overlaps (should be none)
        - Coverage gaps (are all 5 doc files being examined by at least 2 agents?)
        - Redundant work (are two agents doing the same analysis?)
        Approve or send feedback.

Step 5: IMPLEMENTATION PHASE -- re-spawn 4 analyst agents in parallel
        Each reads the LOOM docs, performs its analysis, writes findings.

Step 6: INTEGRATE -- merge all 4 analyst agents in any order (no deps between them)
        Validate after each merge.

Step 7: PLAN + IMPLEMENT dx-report
        dx-report reads MEMORY.md from all 4 analysts.
        Compiles the Developer Experience Report.

Step 8: INTEGRATE dx-report. Clean up all 5 worktrees.
```

## LOOM Features Exercised

| Feature | How |
|---|---|
| Worktree isolation | 5 agents, 5 worktrees, non-overlapping scopes |
| Parallel planning | 4 agents plan simultaneously |
| Plan gate | Orchestrator reviews 4 plans with specific coverage criteria |
| Parallel implementation | 4 analysts implement simultaneously |
| Commit trailers | Every agent commit must have Agent-Id + Session-Id |
| STATUS.md lifecycle | PLANNING -> IMPLEMENTING -> COMPLETED |
| MEMORY.md handoff | 4 analysts write findings; dx-report reads them all |
| Dependency ordering | dx-report waits for all 4 analysts |
| Scope enforcement | Verified at integration time |
| Worktree cleanup | All 5 removed at end |

## Features NOT Tested

- BLOCKED/FAILED states (could emerge organically if an agent finds the docs too confusing to analyze, but not deliberately provoked)
- Resource limit recovery / continuation agents
- Merge conflict recovery
- Heartbeat enforcement
- Cross-agent read of peer STATUS.md

## What This Plan Evaluates That v0 Does Not

Plan v0 tests **protocol correctness** -- do the schemas match, do the commands parse, is the state machine complete. This plan tests **developer experience** -- can a human actually use these docs to get LOOM working? The two are complementary:

- v0 asks "Is the spec internally consistent?"
- v16 asks "Could someone follow this spec without the spec author standing behind them?"

The gap between those two questions is where most developer frustration lives.
