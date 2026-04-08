# LOOM Evaluation Plan v18 -- Protocol Compliance Checker

## Goal

Use LOOM to build a reusable post-hoc validation tool that can inspect any LOOM run's artifacts (branches, commits, worktree files) and report compliance violations against the LOOM protocol specification. The deliverable is a standalone script (`loom-check`) that an orchestrator or human can run after any LOOM session to verify that agents followed the rules.

This plan exercises LOOM as a single-agent task. The interesting evaluation is not parallelism -- it is whether a LOOM agent can read the protocol spec, internalize its rules, and produce a working automated checker that enforces those rules. The checker itself then becomes infrastructure for evaluating all future LOOM runs.

## Agent Decomposition

| Agent ID | Role | Task | Dependencies |
|----------|------|------|-------------|
| `compliance-checker` | Builder | Read the full LOOM protocol spec. Build a validation script that checks all Level 1 compliance rules against real git artifacts. Write tests that exercise each check against synthetic good/bad fixtures. | none |

Single agent. No dependency DAG, no parallel coordination. This plan focuses depth over breadth: one agent, one complex deliverable.

## Scope

| Agent | `paths_allowed` | `paths_denied` |
|-------|-----------------|----------------|
| `compliance-checker` | `tests/loom-eval/compliance-checker/**` | `[]` |

The agent writes all output -- the script, its tests, and its fixtures -- into `tests/loom-eval/compliance-checker/`.

## What the Agent Must Build

### Deliverable: `tests/loom-eval/compliance-checker/loom-check`

A bash or python script (agent chooses) that accepts a git ref range or branch name and validates LOOM protocol compliance. The script must check all of the following:

#### 1. Commit Trailer Validation
- Every commit on `loom/<agent-id>` branches MUST have `Agent-Id` and `Session-Id` trailers.
- `Agent-Id` value must match the `agent_id` in AGENT.json on that branch.
- `Session-Id` value must be a valid UUID v4.
- Commit messages must follow Conventional Commits format: `<type>(<scope>): <subject>`.
- `type` must be one of: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`.

#### 2. STATUS.md Schema Validation
- YAML front matter must parse as valid YAML.
- `status` must be one of: `PLANNING`, `IMPLEMENTING`, `COMPLETED`, `BLOCKED`, `FAILED`.
- Required fields always present: `status`, `updated_at`, `heartbeat_at`, `branch`, `base_commit`, `summary`.
- `files_changed` REQUIRED when status is `COMPLETED`.
- `error` block REQUIRED when status is `FAILED`; MUST NOT be present otherwise.
- `error` sub-fields: `category` (one of 5 values), `message` (string), `retryable` (boolean).
- `blocked_reason` REQUIRED when status is `BLOCKED`; MUST NOT be present otherwise.
- Timestamps must be valid ISO-8601 UTC.

#### 3. MEMORY.md Section Validation
- File must exist when status is `COMPLETED`.
- Must contain all three required sections: `## Key Findings`, `## Decisions`, `## Deviations from Plan`.
- Sections must contain substantive entries (not empty) when status is `COMPLETED`.

#### 4. AGENT.json Schema Validation
- Must parse as valid JSON.
- All required fields present: `agent_id`, `session_id`, `protocol_version`, `context_window_tokens`, `token_budget`, `dependencies`, `scope`, `timeout_seconds`.
- `agent_id` must be kebab-case: `[a-z0-9]+(-[a-z0-9]+)*`.
- `session_id` must be UUID v4 format.
- `protocol_version` must be literal `"loom/1"`.
- `token_budget` must be <= `context_window_tokens`.
- `scope.paths_allowed` must be non-empty.
- `dependencies` must be an array (may be empty).

#### 5. Scope Compliance Validation
- For each commit on the agent branch, diff the changed files against `scope.paths_allowed` and `scope.paths_denied` from AGENT.json.
- Protocol files (STATUS.md, MEMORY.md, PLAN.md, TASK.md, AGENT.json) in the worktree root are always allowed.
- Any file outside allowed globs is a violation. Any file matching a denied glob is a violation (deny takes precedence).

#### 6. Branch Naming Validation
- Agent branches must match pattern `loom/<agent-id>`.
- `<agent-id>` must be kebab-case, max 63 characters.

#### 7. PLAN.md Structure Validation
- Must contain required sections: `## Approach`, `## Steps`, `## Files to Modify`, `## Risks`, `## Estimated Effort`.

### Output Format

The script must produce structured output (one line per check, PASS/FAIL/WARN with details), plus a summary with counts. Example:

```
[PASS] commit-trailers: 12/12 commits have valid Agent-Id and Session-Id
[FAIL] status-schema: STATUS.md missing 'files_changed' field (status is COMPLETED)
[PASS] memory-sections: All 3 required sections present with content
[WARN] scope-compliance: Could not verify -- AGENT.json not found on branch
[PASS] branch-naming: loom/config-parser matches pattern
[PASS] plan-structure: All 5 required sections present

Summary: 4 passed, 1 failed, 1 warning
```

Exit code 0 if all checks pass, 1 if any check fails, 2 if the script itself errors.

### Test Fixtures

The agent must also create test fixtures in `tests/loom-eval/compliance-checker/fixtures/`:

- `good/` -- A minimal set of files representing a compliant LOOM run (valid STATUS.md, MEMORY.md, AGENT.json, PLAN.md, and a mock git log).
- `bad-trailers/` -- Commits missing Agent-Id or Session-Id.
- `bad-status/` -- STATUS.md with COMPLETED but no `files_changed`; FAILED but no `error` block.
- `bad-memory/` -- MEMORY.md missing required sections.
- `bad-scope/` -- Files changed outside allowed paths.
- `bad-agent-json/` -- Invalid AGENT.json (missing fields, bad types).
- `bad-branch/` -- Branch name not matching pattern.

Each fixture directory contains the relevant files. The agent also writes a test runner script (`run-tests.sh` or equivalent) that runs `loom-check` against each fixture and verifies expected PASS/FAIL outcomes.

## Execution Flow

```
Step 1: Create 1 worktree + branch
        git worktree add .worktrees/compliance-checker -b loom/compliance-checker

Step 2: Write TASK.md + AGENT.json into the worktree. Commit.

Step 3: PLANNING PHASE -- spawn compliance-checker
        Agent reads TASK.md, writes PLAN.md, sets STATUS.md to PLANNING, commits, returns.

Step 4: PLAN GATE -- orchestrator reads PLAN.md
        Verify the approach covers all 7 check categories.
        Verify test fixtures are planned for each category.
        Approve or send feedback.

Step 5: IMPLEMENTATION PHASE -- re-spawn compliance-checker
        Agent builds the loom-check script, creates all fixtures, writes test runner.
        Multiple commits as work progresses (one per check category is reasonable).

Step 6: VALIDATE -- orchestrator runs the test runner to confirm the checker works
        bash tests/loom-eval/compliance-checker/run-tests.sh

Step 7: INTEGRATE -- merge loom/compliance-checker into workspace
        Verify STATUS.md is COMPLETED.
        Verify scope compliance (all files in tests/loom-eval/compliance-checker/**).
        git merge --no-ff loom/compliance-checker

Step 8: Clean up worktree
        git worktree remove .worktrees/compliance-checker
```

## LOOM Features Exercised

| Feature | How |
|---|---|
| Worktree isolation | 1 agent, 1 worktree, scoped to `tests/loom-eval/compliance-checker/**` |
| Two-phase lifecycle | Planning spawn, then implementation spawn |
| Plan gate | Orchestrator reviews plan before implementation |
| Commit trailers | Every agent commit must have Agent-Id + Session-Id |
| STATUS.md lifecycle | PLANNING -> IMPLEMENTING -> COMPLETED |
| MEMORY.md handoff | Agent records findings about spec ambiguities and design decisions |
| Scope enforcement | Verified at integration time |
| Worktree cleanup | Removed at end |
| Conventional Commits | All commits follow `<type>(<scope>): <subject>` format |

## LOOM Features NOT Tested

- Parallel agent spawning (single agent plan)
- Dependency DAG and topological integration ordering
- BLOCKED/FAILED states
- Resource limit recovery / continuation agents
- Merge conflict recovery
- Heartbeat enforcement (single agent, short task)
- Agent-to-agent communication via MEMORY.md

## Why This Variant Is Useful

1. **Self-referential evaluation.** The checker validates the very protocol it was built under. After the LOOM run completes, we can run `loom-check` against the `loom/compliance-checker` branch itself -- checking whether the agent that built the checker was itself compliant.

2. **Reusable infrastructure.** Unlike one-off audit plans, this produces a tool. Every future LOOM evaluation plan can include a step: "run `loom-check` against all agent branches" as an automated post-integration gate.

3. **Spec-to-code translation test.** The agent must read the protocol spec (protocol.md, schemas.md, worker-template.md) and translate the MUST/MUST NOT rules into executable checks. This tests the agent's ability to extract formal requirements from natural language specifications -- a qualitatively different challenge from code refactoring or bug fixing.

4. **Fixture design tests planning depth.** The agent must anticipate every failure mode (not just the happy path) and design fixtures that trigger each one. This exercises adversarial thinking during the planning phase.
