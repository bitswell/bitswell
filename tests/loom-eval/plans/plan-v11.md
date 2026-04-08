# LOOM Evaluation Plan v11 -- Schema Fuzzing

## Goal

Verify that the LOOM orchestrator's validation layer correctly detects and rejects every category of malformed protocol artifact. Agents deliberately produce invalid STATUS.md, AGENT.json, MEMORY.md, PLAN.md, TASK.md, commit messages, and branch names. The orchestrator must catch every violation -- a single undetected malformation is a test failure.

This plan treats the orchestrator as a black box under adversarial input. The agents are cooperative (they know the schemas) but their job is to break things on purpose.

## Design Principles

1. **One malformation per test case.** Each agent produces exactly one specific schema violation so failures are attributable to a single root cause.
2. **Coverage over depth.** Every REQUIRED field, every enum value, every conditional constraint, every cross-file invariant gets at least one test case.
3. **The orchestrator is the system under test.** Agents succeed if they produce the malformed artifact. The eval passes if the orchestrator rejects it.

## Agent Decomposition

Seven fuzzing agents, one reporter. Agents are grouped by the artifact they target.

| Agent ID | Role | Target Artifact | Dependencies |
|----------|------|-----------------|-------------|
| `fuzz-status-yaml` | Fuzzer | STATUS.md YAML syntax | none |
| `fuzz-status-schema` | Fuzzer | STATUS.md field values and conditional rules | none |
| `fuzz-agent-json` | Fuzzer | AGENT.json field constraints | none |
| `fuzz-memory` | Fuzzer | MEMORY.md required sections | none |
| `fuzz-plan` | Fuzzer | PLAN.md required sections | none |
| `fuzz-commits` | Fuzzer | Commit message format and trailers | none |
| `fuzz-cross-file` | Fuzzer | Cross-file invariants and branch naming | none |
| `fuzz-report` | Reporter | Compile pass/fail matrix | all 7 fuzzers |

## Scopes

| Agent | `paths_allowed` | `paths_denied` |
|-------|-----------------|----------------|
| `fuzz-status-yaml` | `tests/loom-eval/fuzz-status-yaml/**` | `[]` |
| `fuzz-status-schema` | `tests/loom-eval/fuzz-status-schema/**` | `[]` |
| `fuzz-agent-json` | `tests/loom-eval/fuzz-agent-json/**` | `[]` |
| `fuzz-memory` | `tests/loom-eval/fuzz-memory/**` | `[]` |
| `fuzz-plan` | `tests/loom-eval/fuzz-plan/**` | `[]` |
| `fuzz-commits` | `tests/loom-eval/fuzz-commits/**` | `[]` |
| `fuzz-cross-file` | `tests/loom-eval/fuzz-cross-file/**` | `[]` |
| `fuzz-report` | `tests/loom-eval/fuzz-report/**` | `[]` |

No scope overlap. Each agent writes malformed artifacts as test fixture files within its own directory, not as actual protocol files the orchestrator would consume live. The orchestrator validation logic is invoked programmatically against each fixture.

---

## Complete Malformation Catalog

### A. STATUS.md -- YAML Syntax Violations (`fuzz-status-yaml`)

These test that the orchestrator's YAML parser rejects structurally invalid documents.

| ID | Malformation | Invalid Artifact |
|----|-------------|-----------------|
| SY-01 | Missing opening `---` delimiter | YAML front matter without opening fence |
| SY-02 | Missing closing `---` delimiter | Front matter never terminated |
| SY-03 | Tabs instead of spaces for indentation | `\tstatus: PLANNING` |
| SY-04 | Duplicate keys | Two `status:` lines with different values |
| SY-05 | Bare unquoted special characters in value | `summary: status is: good: very` (ambiguous colons) |
| SY-06 | Invalid YAML type coercion | `status: true` (boolean instead of string) |
| SY-07 | Completely empty front matter | `---\n---` with nothing between delimiters |
| SY-08 | Binary garbage between delimiters | Random bytes injected into YAML block |
| SY-09 | Nested `---` inside front matter | Third `---` line mid-document |
| SY-10 | YAML anchor/alias injection | `status: *alias` referencing undefined anchor |
| SY-11 | No front matter at all | Plain markdown, no `---` delimiters |
| SY-12 | JSON inside YAML delimiters | `---\n{"status": "PLANNING"}\n---` |

### B. STATUS.md -- Field Value and Conditional Rule Violations (`fuzz-status-schema`)

These test that the orchestrator validates field types, required fields, enum values, and conditional presence rules.

| ID | Malformation | Invalid Artifact |
|----|-------------|-----------------|
| SS-01 | Invalid status enum value | `status: RUNNING` (not in enum) |
| SS-02 | Lowercase status enum | `status: planning` (case mismatch) |
| SS-03 | Missing `status` field entirely | All other fields present, `status` omitted |
| SS-04 | Missing `updated_at` field | `status: PLANNING` but no timestamp |
| SS-05 | Missing `heartbeat_at` field | `status: PLANNING` but no heartbeat |
| SS-06 | Missing `branch` field | `status: PLANNING` but no branch |
| SS-07 | Missing `base_commit` field | `status: PLANNING` but no base commit |
| SS-08 | Missing `summary` field | `status: PLANNING` but no summary |
| SS-09 | Invalid `updated_at` format | `updated_at: "April 2, 2026"` (not ISO-8601) |
| SS-10 | Invalid `heartbeat_at` format | `heartbeat_at: 1712345678` (epoch integer, not ISO-8601) |
| SS-11 | `files_changed` missing when COMPLETED | `status: COMPLETED` without `files_changed` |
| SS-12 | `files_changed` present when PLANNING | `status: PLANNING` with `files_changed: 3` (allowed but should test parser tolerance) |
| SS-13 | `files_changed` is negative | `files_changed: -1` |
| SS-14 | `files_changed` is a string | `files_changed: "seven"` |
| SS-15 | `error` block missing when FAILED | `status: FAILED` without `error` object |
| SS-16 | `error` block present when COMPLETED | `status: COMPLETED` with `error:` block (MUST NOT be present) |
| SS-17 | `error` block present when PLANNING | `status: PLANNING` with `error:` block (MUST NOT be present) |
| SS-18 | `error.category` invalid enum | `error.category: timeout` (not in: task_unclear, blocked, resource_limit, conflict, internal) |
| SS-19 | `error.message` missing when FAILED | `error:` present but no `message` sub-field |
| SS-20 | `error.retryable` missing when FAILED | `error:` present but no `retryable` sub-field |
| SS-21 | `error.retryable` is string not boolean | `error.retryable: "yes"` |
| SS-22 | `blocked_reason` missing when BLOCKED | `status: BLOCKED` without `blocked_reason` |
| SS-23 | `blocked_reason` present when COMPLETED | `status: COMPLETED` with `blocked_reason:` (MUST NOT be present) |
| SS-24 | `blocked_reason` present when PLANNING | `status: PLANNING` with `blocked_reason:` |
| SS-25 | `budget.tokens_used` is negative | `budget.tokens_used: -500` |
| SS-26 | `budget.tokens_limit` is zero | `budget.tokens_limit: 0` (must be positive) |
| SS-27 | `budget.tokens_used` exceeds `tokens_limit` | `tokens_used: 200000`, `tokens_limit: 150000` |
| SS-28 | All fields missing (just status) | Only `status: COMPLETED`, nothing else |
| SS-29 | Extra unknown fields | `status: PLANNING` with `mood: happy` (unknown field -- test strict vs. permissive parsing) |
| SS-30 | `heartbeat_at` in the future | `heartbeat_at: "2099-01-01T00:00:00Z"` |

### C. AGENT.json -- Field Constraint Violations (`fuzz-agent-json`)

| ID | Malformation | Invalid Artifact |
|----|-------------|-----------------|
| AJ-01 | Invalid JSON syntax (trailing comma) | `{ "agent_id": "test", }` |
| AJ-02 | Invalid JSON syntax (single quotes) | `{ 'agent_id': 'test' }` |
| AJ-03 | Invalid JSON syntax (unquoted keys) | `{ agent_id: "test" }` |
| AJ-04 | Empty JSON object | `{}` |
| AJ-05 | Missing `agent_id` | All fields except `agent_id` |
| AJ-06 | Missing `session_id` | All fields except `session_id` |
| AJ-07 | Missing `protocol_version` | All fields except `protocol_version` |
| AJ-08 | Missing `context_window_tokens` | All fields except `context_window_tokens` |
| AJ-09 | Missing `token_budget` | All fields except `token_budget` |
| AJ-10 | Missing `dependencies` | All fields except `dependencies` |
| AJ-11 | Missing `scope` | All fields except `scope` |
| AJ-12 | Missing `scope.paths_allowed` | `scope` present but only has `paths_denied` |
| AJ-13 | Missing `scope.paths_denied` | `scope` present but only has `paths_allowed` |
| AJ-14 | Missing `timeout_seconds` | All fields except `timeout_seconds` |
| AJ-15 | `agent_id` not kebab-case (uppercase) | `agent_id: "Config-Parser"` |
| AJ-16 | `agent_id` not kebab-case (underscores) | `agent_id: "config_parser"` |
| AJ-17 | `agent_id` not kebab-case (spaces) | `agent_id: "config parser"` |
| AJ-18 | `agent_id` starts with hyphen | `agent_id: "-config"` |
| AJ-19 | `agent_id` ends with hyphen | `agent_id: "config-"` |
| AJ-20 | `agent_id` empty string | `agent_id: ""` |
| AJ-21 | `agent_id` exceeds 63 characters | 64+ character kebab-case string |
| AJ-22 | `session_id` not UUID v4 | `session_id: "not-a-uuid"` |
| AJ-23 | `session_id` empty string | `session_id: ""` |
| AJ-24 | `protocol_version` wrong value | `protocol_version: "loom/2"` |
| AJ-25 | `protocol_version` empty string | `protocol_version: ""` |
| AJ-26 | `context_window_tokens` is zero | `context_window_tokens: 0` |
| AJ-27 | `context_window_tokens` is negative | `context_window_tokens: -1` |
| AJ-28 | `context_window_tokens` is a string | `context_window_tokens: "200000"` |
| AJ-29 | `context_window_tokens` is a float | `context_window_tokens: 200000.5` |
| AJ-30 | `token_budget` is zero | `token_budget: 0` |
| AJ-31 | `token_budget` exceeds `context_window_tokens` | `token_budget: 300000` with `context_window_tokens: 200000` |
| AJ-32 | `token_budget` is a string | `token_budget: "high"` |
| AJ-33 | `dependencies` is not an array | `dependencies: "agent-parser"` (string instead of array) |
| AJ-34 | `dependencies` contains non-existent agent_id | `dependencies: ["nonexistent-agent"]` |
| AJ-35 | `dependencies` create a cycle | Agent A depends on B, B depends on A |
| AJ-36 | `dependencies` self-reference | `dependencies: ["self-agent"]` where agent_id is "self-agent" |
| AJ-37 | `scope.paths_allowed` is empty array | `paths_allowed: []` (must be non-empty) |
| AJ-38 | `scope.paths_allowed` is a string not array | `paths_allowed: "src/**"` |
| AJ-39 | `timeout_seconds` is zero | `timeout_seconds: 0` |
| AJ-40 | `timeout_seconds` is negative | `timeout_seconds: -3600` |
| AJ-41 | `timeout_seconds` is a string | `timeout_seconds: "1h"` |
| AJ-42 | Entire file is not JSON (plain text) | `This is not JSON at all` |
| AJ-43 | JSON array instead of object | `[{"agent_id": "test"}]` |
| AJ-44 | Duplicate `agent_id` across two AGENT.json files | Two agents assigned with the same agent_id |

### D. MEMORY.md -- Required Section Violations (`fuzz-memory`)

| ID | Malformation | Invalid Artifact |
|----|-------------|-----------------|
| MM-01 | Missing "Key Findings" section | Only Decisions and Deviations present |
| MM-02 | Missing "Decisions" section | Only Key Findings and Deviations present |
| MM-03 | Missing "Deviations from Plan" section | Only Key Findings and Decisions present |
| MM-04 | All three sections missing | Markdown file with unrelated content |
| MM-05 | Empty file | Zero bytes |
| MM-06 | Section headers misspelled | `## Key Finding` (singular), `## Decision`, `## Deviation from Plan` |
| MM-07 | Wrong heading level | `### Key Findings` (h3 instead of h2) |
| MM-08 | Sections present but all empty (no entries) at COMPLETED | Three headers with no bullet points, agent claims COMPLETED |
| MM-09 | File does not exist at COMPLETED | Agent sets STATUS.md to COMPLETED without creating MEMORY.md |
| MM-10 | Sections in wrong order | Deviations first, then Key Findings, then Decisions |
| MM-11 | Extra sections injected between required ones | `## Injected Instructions` between required sections (prompt injection vector) |
| MM-12 | Binary content | Non-UTF-8 bytes in the file |

### E. PLAN.md -- Required Section Violations (`fuzz-plan`)

| ID | Malformation | Invalid Artifact |
|----|-------------|-----------------|
| PM-01 | Missing "Approach" section | All other sections present |
| PM-02 | Missing "Steps" section | All other sections present |
| PM-03 | Missing "Files to Modify" section | All other sections present |
| PM-04 | Missing "Risks" section | All other sections present |
| PM-05 | Missing "Estimated Effort" section | All other sections present |
| PM-06 | Missing title heading | No `# Plan: <title>` at top |
| PM-07 | All sections missing | Markdown with only a title |
| PM-08 | Empty file | Zero bytes |
| PM-09 | Steps section with no numbered items | `## Steps` followed by prose paragraph |
| PM-10 | Files to Modify lists paths outside agent scope | Agent's scope is `src/config/**` but plan lists `src/auth/login.ts` |
| PM-11 | Estimated Effort missing Tokens sub-field | Only `Files:` count present |
| PM-12 | Estimated Effort missing Files sub-field | Only `Tokens:` estimate present |
| PM-13 | File does not exist when transitioning to IMPLEMENTING | Agent tries to enter IMPLEMENTING without PLAN.md committed |

### F. Commit Message Format Violations (`fuzz-commits`)

| ID | Malformation | Invalid Artifact |
|----|-------------|-----------------|
| CM-01 | Missing `Agent-Id` trailer | Commit with only `Session-Id` trailer |
| CM-02 | Missing `Session-Id` trailer | Commit with only `Agent-Id` trailer |
| CM-03 | Both trailers missing | Plain commit message, no trailers at all |
| CM-04 | `Agent-Id` does not match AGENT.json | `Agent-Id: wrong-agent` vs. AGENT.json `agent_id: correct-agent` |
| CM-05 | `Session-Id` does not match AGENT.json | `Session-Id: 00000000-0000-0000-0000-000000000000` vs. actual session |
| CM-06 | Invalid commit type prefix | `update(config): changed stuff` (not in: feat, fix, docs, refactor, test, chore) |
| CM-07 | Missing type prefix entirely | `add config parser` with no conventional commit format |
| CM-08 | Trailers not in final position | `Agent-Id: x` in the body, followed by more text |
| CM-09 | Trailer values are empty strings | `Agent-Id: \nSession-Id: ` |
| CM-10 | No blank line between body and trailers | Body text runs directly into `Agent-Id:` |
| CM-11 | Trailers duplicated with different values | Two `Agent-Id:` lines with different IDs |
| CM-12 | Subject line empty | `feat(config): \n\nAgent-Id: ...` |

### G. Cross-File Invariants and Branch Naming (`fuzz-cross-file`)

These test constraints that span multiple files or relate to git-level rules.

| ID | Malformation | Invalid Artifact |
|----|-------------|-----------------|
| XF-01 | Branch name does not match `loom/<agent-id>` pattern | Branch is `feature/my-agent` instead of `loom/my-agent` |
| XF-02 | Branch name uses uppercase | `loom/Config-Parser` |
| XF-03 | `STATUS.md` branch field does not match actual branch | `branch: loom/wrong-name` on branch `loom/correct-name` |
| XF-04 | `STATUS.md` base_commit does not match actual branch point | `base_commit: 0000000000` (fabricated SHA) |
| XF-05 | `AGENT.json` agent_id does not match branch suffix | `agent_id: "alpha"` on branch `loom/beta` |
| XF-06 | `TASK.md` scope does not match `AGENT.json` scope | TASK.md says `src/config/**` allowed, AGENT.json says `src/auth/**` |
| XF-07 | Agent modifies files outside its scope | Commits touching `src/auth/login.ts` when scope only allows `src/config/**` |
| XF-08 | Agent modifies files in `paths_denied` | Commits touching a path explicitly listed in `scope.paths_denied` |
| XF-09 | Agent modifies another agent's worktree | Writes to `.worktrees/other-agent/` |
| XF-10 | Agent pushes to workspace (main working tree) | Writes directly to the orchestrator's workspace |
| XF-11 | Invalid state transition: PLANNING -> COMPLETED | Skips IMPLEMENTING |
| XF-12 | Invalid state transition: IMPLEMENTING -> PLANNING | Goes backwards |
| XF-13 | Invalid state transition: COMPLETED -> IMPLEMENTING | Leaves terminal state |
| XF-14 | Invalid state transition: FAILED -> IMPLEMENTING | Leaves terminal state |
| XF-15 | TASK.md mutated by agent after spawn | Agent modifies TASK.md (should be immutable) |
| XF-16 | AGENT.json mutated by agent | Agent overwrites its own AGENT.json |
| XF-17 | Dependency not integrated before dependent agent's integration | Orchestrator tries to integrate dependent before dependency |
| XF-18 | `files_changed` count does not match actual changed files | STATUS.md says `files_changed: 2` but diff shows 5 files |

---

## Execution Flow

```
Step 1: Create 8 worktrees + branches
        git worktree add .worktrees/fuzz-status-yaml    -b loom/fuzz-status-yaml
        git worktree add .worktrees/fuzz-status-schema  -b loom/fuzz-status-schema
        git worktree add .worktrees/fuzz-agent-json     -b loom/fuzz-agent-json
        git worktree add .worktrees/fuzz-memory         -b loom/fuzz-memory
        git worktree add .worktrees/fuzz-plan           -b loom/fuzz-plan
        git worktree add .worktrees/fuzz-commits        -b loom/fuzz-commits
        git worktree add .worktrees/fuzz-cross-file     -b loom/fuzz-cross-file
        git worktree add .worktrees/fuzz-report         -b loom/fuzz-report

Step 2: Write TASK.md + AGENT.json into each worktree. Commit.
        Each fuzzer's TASK.md contains its section of the malformation catalog
        (the relevant table from this plan) as acceptance criteria.

Step 3: PLANNING PHASE -- spawn all 7 fuzzer agents in parallel
        Each writes PLAN.md describing how it will produce its fixture files
        and what validation checks it expects the orchestrator to perform.

Step 4: PLAN GATE -- orchestrator reads all 7 PLAN.md files
        Verify each fuzzer covers every test case in its assigned table.
        Verify no scope overlaps. Approve or provide feedback.

Step 5: IMPLEMENTATION PHASE -- spawn all 7 fuzzer agents in parallel
        Each agent:
          a. Creates a directory of numbered fixture files (one per test case).
          b. Creates a test-manifest.json listing each fixture, the expected
             validation error, and the relevant schema rule reference.
          c. Writes MEMORY.md documenting any ambiguities found in the schema.
          d. Sets STATUS.md to COMPLETED.

Step 6: INTEGRATE -- merge all 7 fuzzers in any order (no deps)
        Validate after each merge.

Step 7: PLAN + IMPLEMENT fuzz-report (depends on all 7 fuzzers)
        Reads all 7 test-manifest.json files.
        Compiles a consolidated pass/fail matrix:
          - Total test cases across all categories
          - For each: test ID, target artifact, malformation type,
            expected error, and a stub for actual result (to be filled
            when orchestrator validation is run against fixtures)
        Writes the matrix as tests/loom-eval/fuzz-report/validation-matrix.md.

Step 8: INTEGRATE fuzz-report. Clean up all worktrees.
```

## Fixture File Convention

Each fuzzer agent produces fixtures in a consistent format:

```
tests/loom-eval/fuzz-<target>/
  fixtures/
    SY-01-missing-opening-delimiter.md      # The malformed artifact
    SY-02-missing-closing-delimiter.md
    ...
  test-manifest.json                        # Machine-readable index
```

### test-manifest.json schema

```json
{
  "agent": "<fuzzer-agent-id>",
  "target_artifact": "<STATUS.md | AGENT.json | MEMORY.md | PLAN.md | commit | branch>",
  "test_cases": [
    {
      "id": "SY-01",
      "fixture": "fixtures/SY-01-missing-opening-delimiter.md",
      "malformation": "Missing opening --- delimiter in YAML front matter",
      "schema_rule": "STATUS.md YAML front matter delimited by ---. Parsers MUST treat the block as YAML.",
      "expected_error": "yaml_parse_error",
      "severity": "reject"
    }
  ]
}
```

`severity` is one of:
- `reject` -- orchestrator MUST reject this artifact and refuse to proceed
- `warn` -- orchestrator SHOULD flag but may tolerate (for ambiguous cases like SS-29 unknown fields)

## LOOM Features Exercised

| Feature | How |
|---|---|
| Worktree isolation | 8 agents, 8 worktrees, non-overlapping scopes |
| Parallel planning | 7 agents plan simultaneously |
| Plan gate | Orchestrator reviews 7 plans before any implementation |
| Parallel implementation | 7 agents implement simultaneously |
| Commit trailers | Every agent commit must have Agent-Id + Session-Id |
| STATUS.md lifecycle | PLANNING -> IMPLEMENTING -> COMPLETED for all agents |
| MEMORY.md handoff | Fuzzers write ambiguity findings; reporter reads them |
| Dependency ordering | fuzz-report waits for all 7 fuzzers |
| Scope enforcement | Verified at integration time |
| Worktree cleanup | All 8 removed at end |

## Features NOT Tested by This Plan

- BLOCKED/FAILED states (agents all succeed; the malformations are in fixture files, not in agent protocol compliance)
- Resource limit recovery / continuation agents
- Merge conflict recovery
- Heartbeat enforcement
- Actual orchestrator validation execution (this plan produces the test corpus; running the corpus against orchestrator validation is a separate step)

## Test Case Summary

| Category | Agent | Count |
|----------|-------|-------|
| A. STATUS.md YAML syntax | `fuzz-status-yaml` | 12 |
| B. STATUS.md field values / conditionals | `fuzz-status-schema` | 30 |
| C. AGENT.json field constraints | `fuzz-agent-json` | 44 |
| D. MEMORY.md required sections | `fuzz-memory` | 12 |
| E. PLAN.md required sections | `fuzz-plan` | 13 |
| F. Commit message format | `fuzz-commits` | 12 |
| G. Cross-file invariants / branches | `fuzz-cross-file` | 18 |
| **Total** | | **141** |

## Success Criteria

1. All 141 fixture files are produced and indexed in test-manifest.json files.
2. The validation-matrix.md report accounts for every test case.
3. When orchestrator validation is run against each fixture, all 141 cases with severity `reject` are caught. Zero false negatives.
4. Cases marked `warn` are documented with rationale for why strict vs. permissive parsing is appropriate.
