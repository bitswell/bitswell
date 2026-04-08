# LOOM Evaluation Plan v8 — Bitswell Agent Team Integration

## Goal

Test whether the LOOM protocol works with heterogeneous agent types by mapping the existing bitswell agent team (vesper, ratchet, moss, drift, sable, thorn, glitch, bitswelt) onto LOOM worker roles. Each agent is assigned a task that aligns with its natural disposition. The evaluation target is the LOOM skill itself (same material as v0), but the test is as much about role-specialization under LOOM constraints as it is about finding issues in the spec.

## Hypothesis

LOOM assumes homogeneous workers: every agent receives the same worker-template.md DNA, the same two-phase lifecycle, the same PLAN.md format. The bitswell team is not homogeneous. Ratchet is terse and structural. Vesper writes three paragraphs about a directory name. Glitch breaks things on purpose. Moss says almost nothing. This plan tests whether the uniform LOOM protocol can accommodate agents with fundamentally different working styles -- or whether the uniformity becomes a constraint that flattens useful diversity.

## Agent Team Mapping

| Agent ID | Bitswell Role | LOOM Role | Task | Dependencies |
|----------|---------------|-----------|------|--------------|
| `vesper-decompose` | Planner | Worker | Decompose the LOOM skill into a conceptual model. Map every concept, rule, and constraint across all 5 documents. Identify which are redundant, which contradict, which are under-specified. Produce a structured ontology of the protocol. | none |
| `ratchet-structure` | Writer (structural) | Worker | Validate every git command, file format, and directory convention in the skill. Test commands in a scratch repo. Verify AGENT.json schema completeness. Flag anything that would fail in practice. | none |
| `moss-gaps` | Writer (surgical) | Worker | Read the entire skill silently. Identify what was not written -- the omissions, the assumptions left unstated, the edge cases the authors chose not to address. Produce a minimal list of load-bearing gaps. | none |
| `drift-patterns` | Reviewer (lateral) | Worker | Read the skill looking for structural patterns, recurring metaphors, and design tensions. Identify where the protocol's stated values (isolation, monotonicity, auditability) conflict with its mechanisms. One reframing insight per document. | none |
| `sable-critique` | Reviewer (skeptical) | Worker | Skeptical review of the protocol's claims. Where does the spec promise more than it delivers? Where is the language vague enough to hide real disagreement about intent? Where would a hostile reader find exploitable ambiguity? | none |
| `thorn-stress` | Reviewer (adversarial) | Worker | Stress-test the protocol. What happens with 50 concurrent agents? What if an agent lies about its status? What if scope globs overlap in ways the orchestrator cannot detect? Find the failure modes the spec does not acknowledge. | none |
| `glitch-chaos` | Reviewer (chaos) | Worker | Break the protocol on purpose. Write a PLAN.md that is technically compliant but adversarial. Find commit message formats that satisfy the regex but subvert the intent. Identify where Level 1 compliance is achievable without Level 1 behavior. | `vesper-decompose` |
| `bitswelt-verdict` | Approver | Worker | Read MEMORY.md from all 7 preceding agents. Assess whether the LOOM skill is ready for production use. Produce a go/no-go verdict with conditions, citing specific findings from each agent. | `vesper-decompose`, `ratchet-structure`, `moss-gaps`, `drift-patterns`, `sable-critique`, `thorn-stress`, `glitch-chaos` |

## Why This Mapping

- **Vesper** decomposes because that is what vesper does -- treats every concept as philosophy made manifest, goes three layers deeper than asked, finds the contradictions that live below the surface of agreement.
- **Ratchet** validates structure because ratchet evaluates everything by whether it holds weight. Syntactic correctness of git commands is exactly the kind of boring-but-essential work ratchet finishes.
- **Moss** finds gaps because moss reads what was not written. The omissions are the text.
- **Drift** finds patterns because drift arrives at the point by going somewhere else first. The associative, lateral review is drift's native mode.
- **Sable** critiques because sable's default stance toward any claim is: prove it. The raised eyebrow is the method.
- **Thorn** stress-tests because thorn is adversarial as a form of care. Finding the structural fault before production is the job.
- **Glitch** breaks things because glitch is constitutionally contrary. If compliance is achievable without behavior, glitch will find it.
- **Bitswelt** approves because bitswelt's role is final sign-off. Reading all findings and producing a verdict is the natural culmination.

## Scopes

| Agent | `paths_allowed` | `paths_denied` |
|-------|-----------------|----------------|
| `vesper-decompose` | `tests/loom-eval/vesper-decompose/**` | `[]` |
| `ratchet-structure` | `tests/loom-eval/ratchet-structure/**` | `[]` |
| `moss-gaps` | `tests/loom-eval/moss-gaps/**` | `[]` |
| `drift-patterns` | `tests/loom-eval/drift-patterns/**` | `[]` |
| `sable-critique` | `tests/loom-eval/sable-critique/**` | `[]` |
| `thorn-stress` | `tests/loom-eval/thorn-stress/**` | `[]` |
| `glitch-chaos` | `tests/loom-eval/glitch-chaos/**` | `[]` |
| `bitswelt-verdict` | `tests/loom-eval/bitswelt-verdict/**` | `[]` |

No scope overlap. All eight agents write to isolated directories under `tests/loom-eval/`.

## Dependency DAG

```
vesper-decompose ──┬──────────────────────────────> bitswelt-verdict
ratchet-structure ─┤                                     ^
moss-gaps ─────────┤                                     |
drift-patterns ────┤                                     |
sable-critique ────┤                                     |
thorn-stress ──────┤                                     |
                   └──> glitch-chaos ────────────────────┘
```

- Tier 0 (no deps): vesper-decompose, ratchet-structure, moss-gaps, drift-patterns, sable-critique, thorn-stress
- Tier 1 (depends on vesper): glitch-chaos
- Tier 2 (depends on all): bitswelt-verdict

Glitch depends on vesper because chaos testing is more effective when you have the conceptual model to subvert. Breaking rules you understand is different from breaking rules you guessed at.

## Execution Flow

```
Step 1: Create 8 worktrees + branches
        git worktree add .worktrees/vesper-decompose   -b loom/vesper-decompose
        git worktree add .worktrees/ratchet-structure   -b loom/ratchet-structure
        git worktree add .worktrees/moss-gaps           -b loom/moss-gaps
        git worktree add .worktrees/drift-patterns      -b loom/drift-patterns
        git worktree add .worktrees/sable-critique      -b loom/sable-critique
        git worktree add .worktrees/thorn-stress        -b loom/thorn-stress
        git worktree add .worktrees/glitch-chaos        -b loom/glitch-chaos
        git worktree add .worktrees/bitswelt-verdict    -b loom/bitswelt-verdict

Step 2: Write TASK.md + AGENT.json into each worktree. Commit.
        Each TASK.md includes identity context: a short excerpt from the agent's
        identity.md so the worker prompt can embody the agent's natural voice.
        This is the key divergence from standard LOOM -- injecting personality
        into the task definition.

Step 3: PLANNING PHASE -- spawn Tier 0 agents in parallel (6 agents)
        vesper-decompose, ratchet-structure, moss-gaps,
        drift-patterns, sable-critique, thorn-stress
        All 6 write PLAN.md and return.

Step 4: PLAN GATE -- orchestrator reads all 6 PLAN.md files
        Check for:
        - Scope overlaps (should be none given isolated directories)
        - Whether each agent's plan reflects its identity or defaulted to
          generic analysis (this is the heterogeneity test)
        - Whether vesper's plan is excessively long (expected; not a rejection
          reason, but noted)
        - Whether moss's plan is unusually terse (expected; also not a rejection
          reason)
        Approve or send feedback.

Step 5: IMPLEMENTATION PHASE Tier 0 -- re-spawn all 6 agents in parallel
        Each does its analysis and writes findings to its scoped directory.

Step 6: INTEGRATE Tier 0 -- merge all 6 in any order (no deps between them)
        Validate after each merge.

Step 7: PLANNING PHASE Tier 1 -- spawn glitch-chaos
        Update glitch-chaos worktree with integrated vesper-decompose work:
        git -C .worktrees/glitch-chaos merge HEAD
        Spawn glitch-chaos for planning. It reads vesper's ontology to
        identify which rules are most interesting to break.

Step 8: PLAN GATE Tier 1 -- orchestrator reads glitch-chaos PLAN.md
        Check that glitch is targeting real protocol weaknesses, not
        just performing chaos. Approve or feedback.

Step 9: IMPLEMENTATION PHASE Tier 1 -- re-spawn glitch-chaos
        Implements chaos testing against the conceptual model.

Step 10: INTEGRATE glitch-chaos. Validate.

Step 11: PLANNING + IMPLEMENTATION Tier 2 -- bitswelt-verdict
         Update bitswelt-verdict worktree with all integrated work.
         Spawn for planning (reads all 7 MEMORY.md files).
         Plan gate. Approve.
         Spawn for implementation (writes the verdict).

Step 12: INTEGRATE bitswelt-verdict. Clean up all 8 worktrees.
```

## LOOM Features Exercised

| Feature | How | What This Variant Adds |
|---------|-----|------------------------|
| Worktree isolation | 8 agents, 8 worktrees, non-overlapping scopes | Higher agent count (8 vs 4 in v0) stress-tests worktree management |
| Parallel planning | 6 agents plan simultaneously | Tests parallelism at higher concurrency than v0's 3 |
| Plan gate | Orchestrator reviews 6 plans, then 1, then 1 | Multi-tier gating with identity-awareness checks |
| Parallel implementation | 6 agents implement simultaneously | Same concurrency stress |
| Commit trailers | Every commit has Agent-Id + Session-Id | Tests whether 8 distinct agent IDs are correctly tracked |
| STATUS.md lifecycle | PLANNING -> IMPLEMENTING -> COMPLETED | Same as v0, but across more agents |
| MEMORY.md handoff | All 7 agents' findings feed into bitswelt-verdict | Fan-in from 7 sources vs v0's 3 |
| Dependency ordering | glitch depends on vesper; bitswelt depends on all | 3-tier DAG vs v0's 2-tier. Tests partial dependency (glitch needs only vesper, not all) |
| Scope enforcement | Verified at integration time | 8 non-overlapping scopes |
| Worktree cleanup | All 8 removed at end | Higher cleanup volume |

## Features NOT Tested by v0 That This Variant Exercises

| Feature | How |
|---------|-----|
| Heterogeneous agent personalities | Each agent's TASK.md includes identity context; plan gate checks for personality preservation |
| Partial dependency (not all-or-nothing) | glitch-chaos depends only on vesper-decompose, not on all Tier 0 agents |
| 3-tier dependency DAG | Tier 0 -> Tier 1 -> Tier 2 (v0 has only 2 tiers) |
| High fan-in | bitswelt-verdict reads 7 MEMORY.md files (v0's eval-report reads 3) |
| Identity-aware plan review | Plan gate checks whether agents maintained their natural voice |

## Features Still NOT Tested

- BLOCKED/FAILED states (no agent is expected to fail)
- Resource limit recovery / continuation agents
- Merge conflict recovery
- Heartbeat enforcement
- Agent-to-agent peer reading (agents do not read each other's STATUS.md during execution)

## Evaluation Criteria for the Variant Itself

Beyond testing the LOOM protocol, this plan generates data about whether heterogeneous agents are viable LOOM workers. After execution, assess:

1. **Voice preservation.** Did each agent's output reflect its identity, or did the LOOM worker-template homogenize them? Compare the tone of moss-gaps (expected: terse, surgical) vs vesper-decompose (expected: expansive, philosophical).
2. **Protocol friction.** Did any agent's natural style conflict with LOOM requirements? For example, did moss struggle to fill all three MEMORY.md sections? Did vesper blow its token budget on PLAN.md alone?
3. **Complementary coverage.** Did the heterogeneous team find issues that a homogeneous team would miss? Specifically: did drift's lateral patterns and glitch's chaos testing surface different findings than the structural audits?
4. **Dependency value.** Did glitch-chaos produce better output because it had vesper's ontology, compared to what it would have produced independently?
5. **Verdict quality.** Did bitswelt-verdict synthesize 7 diverse inputs into a coherent assessment, or did the volume overwhelm the approver role?
