# LOOM Evaluation Plan v7 -- Self-Improvement

## Goal

Evaluate the LOOM skill by having LOOM agents analyze the skill files themselves and produce concrete patches that improve them. The deliverable is not a report but an actually-improved LOOM skill. This is meta-evaluation: if LOOM can coordinate agents to fix its own specification, the protocol works. If the agents fail, their failure modes reveal exactly what needs fixing.

## Rationale

Plans v0-v6 treat the LOOM skill as a read-only subject of analysis. This plan treats it as a codebase to be improved. Agents must deeply read, understand, cross-reference, and modify the five skill files. Any bug they hit while following the protocol to improve the protocol is a finding of the highest fidelity.

## Agent Decomposition

| Agent ID | Role | Task | Dependencies |
|----------|------|------|-------------|
| `consistency-fixer` | Patch author | Cross-reference all five files for inconsistencies (conflicting field names, mismatched schemas, contradictory rules). Produce patches to reconcile them. | none |
| `gap-filler` | Patch author | Identify underspecified areas: missing error handling paths, undefined behavior at state boundaries, ambiguous MUST/SHOULD language. Write the missing content. | none |
| `example-fixer` | Patch author | Validate every code block in all five files for syntactic correctness, match examples against their schemas, fix broken or misleading examples. | none |
| `clarity-editor` | Patch author | Rewrite ambiguous passages, add cross-references between files where concepts are introduced in one place but used in another without linkage, fix structural issues. | none |
| `integration-patcher` | Integrator | Read all four agents' patches. Resolve overlapping edits. Merge non-conflicting patches into a single coherent changeset across all five files. Produce the final improved skill. | `consistency-fixer`, `gap-filler`, `example-fixer`, `clarity-editor` |

Five agents. Four independent patch authors, one dependent integrator.

## Scopes

| Agent | `paths_allowed` | `paths_denied` |
|-------|-----------------|----------------|
| `consistency-fixer` | `tests/loom-eval/self-improve/consistency/**` | `[]` |
| `gap-filler` | `tests/loom-eval/self-improve/gaps/**` | `[]` |
| `example-fixer` | `tests/loom-eval/self-improve/examples/**` | `[]` |
| `clarity-editor` | `tests/loom-eval/self-improve/clarity/**` | `[]` |
| `integration-patcher` | `tests/loom-eval/self-improve/integrated/**`, `.claude/skills/loom/**` | `[]` |

The four patch authors write to isolated staging directories. Only the integration agent has write access to the actual skill files. This prevents partial or conflicting edits from landing piecemeal.

## Output Format for Patch Agents

Each patch agent produces these files in its scope directory:

```
<scope-dir>/
  FINDINGS.md       # What was found wrong, with file:line references
  PATCHES.md        # Proposed changes as unified-diff-style blocks
  RATIONALE.md      # Why each change is correct, referencing spec sections
```

### PATCHES.md Format

Each patch is a fenced block identifying the target file, the original text, and the replacement:

```markdown
### Patch 1: <short title>

**File**: `.claude/skills/loom/references/schemas.md`
**Section**: 3. STATUS.md YAML Schema

Original:
\`\`\`
<exact text to replace>
\`\`\`

Replacement:
\`\`\`
<corrected text>
\`\`\`

**Justification**: <Why this change is needed, cross-referencing the conflicting source>
```

This format is machine-readable enough for the integration agent to apply patches, and human-readable enough to review.

## Known Issues to Seed Agent Tasks

These are starter leads, not exhaustive lists. Agents must find additional issues on their own.

### consistency-fixer leads

- SKILL.md step 9 says "verify scope, merge --no-ff" but does not mention the scope-compliance diff check shown in Command Patterns. Are these the same operation or different?
- protocol.md Section 5.2 says commit precondition is "PLANNING or IMPLEMENTING" but worker-template.md has agents committing STATUS.md during the PLANNING->IMPLEMENTING transition. Is this a commit in PLANNING state or IMPLEMENTING state?
- schemas.md says `files_changed` is "REQUIRED when COMPLETED" but the worker-template STATUS.md examples show `files_changed: 0` in the PLANNING example. Is it always-required or only-when-COMPLETED?
- The `budget` block is described in protocol.md Section 4.1 and schemas.md Section 3 with slightly different wording about when it is required.

### gap-filler leads

- No specification for what happens if the orchestrator crashes mid-integration. The workspace may be in a partially merged state.
- No specification for agent-id uniqueness enforcement across time. Can a new agent reuse the id of a completed agent?
- The heartbeat mechanism (5-minute commits) has no specification for what happens if the git repository's clock is wrong or if the agent cannot commit (e.g., disk full).
- No specification for maximum task decomposition depth (can an orchestrator spawn sub-orchestrators?).
- The `store` operation (Section 5.4) at Level 2 references `refs/loom/memory/` but no other file mentions this ref namespace.

### example-fixer leads

- Example 1 creates the branch separately (`git branch loom/... HEAD`) then attaches a worktree, while SKILL.md uses `git worktree add ... -b loom/...` which creates branch and worktree atomically. The two patterns have different failure modes.
- Example 3 uses `git merge HEAD` inside the validator worktree to pull in integrated parser work. This merges the worktree's own HEAD into itself, which is a no-op. The intended command is likely `git merge main` or `git merge <workspace-branch>`.
- Example 4 says the agent sets `status: blocked` with `blocked_reason: resource_limit`, but the error procedure in worker-template.md says to exit with code 1 after setting BLOCKED. The error procedure section says exit 1 is for FAILED, not BLOCKED. Contradiction.
- Commit messages in examples sometimes have a blank line before trailers and sometimes do not. The schema says the format requires a blank line between body and trailers, but some examples omit the body entirely -- is a blank line still required?

### clarity-editor leads

- The term "workspace" is defined in protocol.md but used without definition in SKILL.md. A reader starting from SKILL.md may not know what "workspace" means versus "worktree".
- SKILL.md says "The Agent tool is blocking, so use two-phase spawn" but never explains what the Agent tool is. A cross-reference to the Claude Code documentation or a brief explanation is needed.
- The relationship between SKILL.md (the orchestrator's playbook) and worker-template.md (the worker's DNA) is not explained anywhere. A reader may not understand which document governs which role.
- Level 1 vs Level 2 conformance is referenced in multiple files but Level 2 is never defined beyond "Level 2+" requirements for the budget block.

## Execution Flow

```
Step 1: Create 5 worktrees + branches
        git worktree add .worktrees/consistency-fixer    -b loom/consistency-fixer
        git worktree add .worktrees/gap-filler           -b loom/gap-filler
        git worktree add .worktrees/example-fixer        -b loom/example-fixer
        git worktree add .worktrees/clarity-editor       -b loom/clarity-editor
        git worktree add .worktrees/integration-patcher  -b loom/integration-patcher

Step 2: Write TASK.md + AGENT.json into each worktree.
        Each TASK.md includes:
          - The full text of all 5 LOOM skill files (read-only context)
          - The specific analysis mandate for that agent
          - The seed leads listed above as starting points
          - The PATCHES.md output format specification
        Commit.

Step 3: PLANNING PHASE -- spawn 4 patch agents in parallel
        consistency-fixer, gap-filler, example-fixer, clarity-editor
        Each reads the skill files, analyzes their domain, writes PLAN.md
        listing the issues they intend to address and patches they will draft.

Step 4: PLAN GATE -- orchestrator reads all 4 PLAN.md files
        Check for:
          - Overlapping patches (two agents patching the same passage)
          - Missing coverage (known issues not claimed by any agent)
          - Scope creep (agent proposing rewrites beyond their mandate)
        If overlap exists: assign the passage to one agent, tell the other
        to skip it via Feedback. This is critical -- overlapping patches
        will create merge conflicts for the integration agent.

Step 5: IMPLEMENTATION PHASE -- re-spawn 4 patch agents in parallel
        Each produces FINDINGS.md, PATCHES.md, RATIONALE.md in its directory.
        Each writes MEMORY.md summarizing its most important fixes.

Step 6: INTEGRATE 4 patch agents in any order (no deps between them)
        Validate after each merge.

Step 7: PLAN integration-patcher
        TASK.md includes:
          - Read all 4 agents' PATCHES.md and RATIONALE.md files
          - Apply non-conflicting patches to the actual skill files
          - For conflicting patches, choose the better fix and document why
          - Run a final consistency check: do the patched files still agree?
          - Produce a CHANGELOG.md listing every change made

Step 8: PLAN GATE for integration-patcher
        Review the plan. Ensure it does not propose deleting content or
        making structural changes beyond what the patches call for.

Step 9: IMPLEMENT integration-patcher
        The agent applies patches to the actual .claude/skills/loom/ files.
        Produces tests/loom-eval/self-improve/integrated/CHANGELOG.md.

Step 10: INTEGRATE integration-patcher. Clean up all worktrees.
```

## LOOM Features Exercised

| Feature | How |
|---|---|
| Worktree isolation | 5 agents, 5 worktrees, non-overlapping scopes |
| Parallel planning | 4 agents plan simultaneously |
| Plan gate with deconfliction | Orchestrator must resolve overlapping patches at plan gate |
| Parallel implementation | 4 agents implement simultaneously |
| Commit trailers | Every agent commit must have Agent-Id + Session-Id |
| STATUS.md lifecycle | PLANNING -> IMPLEMENTING -> COMPLETED |
| MEMORY.md handoff | Patch agents write findings; integration agent reads them |
| Dependency ordering | integration-patcher waits for all 4 patch agents |
| Scope enforcement | Only integration-patcher may write to actual skill files |
| Worktree cleanup | All 5 removed at end |
| Cross-file analysis | Agents must read and cross-reference 5 files to find issues |
| Concrete output | Deliverable is a patched skill, not just a report |

## Features NOT Tested

- BLOCKED/FAILED states (unless an agent actually hits resource limits)
- Resource limit recovery / continuation agents
- Merge conflict recovery (unless patches conflict at integration)
- Heartbeat enforcement
- Sub-orchestrator spawning

## Success Criteria

The evaluation succeeds if:

1. **Patches land.** The integration agent produces a modified set of LOOM skill files with at least 10 concrete fixes applied.
2. **Patches are correct.** Each fix resolves a real inconsistency, gap, or error -- not a cosmetic preference. The RATIONALE.md entries hold up to scrutiny.
3. **No regressions.** The patched skill files do not introduce new inconsistencies. The integration agent's final consistency check passes.
4. **Protocol was followed.** All 5 agents followed LOOM Level 1 compliance rules throughout. Any protocol violations encountered by the agents during execution are themselves findings that inform additional patches.
5. **The skill is measurably better.** A before/after diff shows the skill files are more internally consistent, more complete, and less ambiguous.

## Why This Variant Matters

Self-improvement is the hardest possible evaluation because:

- Agents must deeply understand the subject matter (the LOOM protocol) to critique it.
- Agents must follow the protocol while simultaneously finding its flaws -- if the protocol has a bug that trips them up, that is a finding.
- The output is verifiable: patches either fix real problems or they do not.
- The evaluation produces lasting value: the improved skill files can be used going forward.
- Overlapping-patch deconfliction at the plan gate exercises a nuance of LOOM coordination that report-based evaluations never touch.
