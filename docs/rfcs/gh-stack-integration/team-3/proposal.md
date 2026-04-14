# Team 3 — gh-stack as a First-Class Protocol Primitive

**RFP:** Epic #74 — gh-stack integration with LOOM
**Angle:** Maximum-protocol-change. Extend the LOOM commit-trailer vocabulary
with new `Stack-*` trailers so stack membership, position, and base are
wire-format facts the dispatcher, validators, and integrator all reason about
directly — rather than a convention layered on top of the audit trail after
the fact.

---

## 1. Angle statement

Stack topology is a wire-format fact. Every LOOM commit that participates in
a stacked epic carries `Stack-Id`, `Stack-Position`, `Stack-Base`, and
`Stack-Epic` trailers, and the existing validators (`trailer-validate`,
`lifecycle-check`, `dag-check`, `dispatch`) enforce stack invariants the same
way they enforce lifecycle invariants today — by reading git.

This is explicitly the **maximum** protocol-change angle. Where other teams
try to leave the schema frozen and wedge `gh-stack` onto it as a publisher,
a recipe, a sidecar, or a naming convention, Team 3's thesis is that paying
a small, bounded schema debt now is strictly cheaper than paying an
unbounded tooling-drift debt forever. The non-negotiable claim: if an epic
is stack-mode, every ASSIGNED commit for its workers MUST carry a `Stack-Id`
and a `Stack-Position`, and `trailer-validate.ts` MUST reject an assignment
that omits them.

Shippability is non-negotiable too. All four new trailers are
**conditional on stack-mode**. A non-stack epic never sees them, a legacy
validator that does not know about them ignores them as unknown trailers per
`git-interpret-trailers(1)` semantics, and the rollout is additive and
reversible. This is the opposite of a wire-break.

---

## 2. Thesis

### 2.1 Why encode stacks in the protocol

LOOM's most distinctive property is that git is the database. Every
invariant the orchestrator cares about — who is working on what, which
files they are allowed to touch, whether a dependency is satisfied, how
long they have been running, what they learned — is expressed as a commit
trailer, read back by a validator, and enforced by rejecting commits or
refusing to dispatch. There is no sidecar YAML, no Redis, no
`.loom/state.json`. The audit trail *is* the state.

The RFP asks us to integrate a tool — `gh-stack` — whose model is
**fundamentally incompatible with that property as currently stated**.
`gh-stack` stores stack topology in its own stack file, rebases branches
with `--force-with-lease`, and rewrites history on every sync. LOOM
integrates with `--no-ff` merges and never rewrites history on `loom/*`
branches. These are not small impedance mismatches; they are two different
theories of where truth lives.

Three of the four non-Team-3 angles try to keep the LOOM wire format frozen
and bolt `gh-stack` on sideways. Each pushes the cost of "what is a stack"
into tooling that re-derives it on every invocation:

- **As a publisher.** Team 1's angle (projection-layer only) treats
  `gh-stack` branches as a disposable output format. It works for happy
  paths but fails the moment a reviewer asks "which `loom/*` branch
  corresponds to PR #42?" because the mapping lives only in the publisher's
  memory.
- **As a recipe.** A workflow document that says "to do a stack, do these
  seven things in order" has no enforcement. An agent that forgets step 4
  produces a broken stack, and the orchestrator has no way to detect it
  until the reviewer notices.
- **As a convention.** Encoding position in the branch name
  (`loom/<agent>-<slug>-s01`) overloads the branch namespace with topology.
  Every rework forces a branch rename. Every validator that currently
  parses branch names needs a new code path.

All three of these bolt-on approaches leak on the **same class of hard
cases**: mid-stack rework, DAG fan-out that is not a pure linear chain,
and workers that need to know their layer to write correct code. These
cases stop being hard the moment stack topology is a trailer — because
trailers are trivially introspectable, validated at commit time, and
propagated through the existing `Session-Id`-style auto-inject machinery.

### 2.2 Why trailers are the right abstraction

Trailers are LOOM's existing extension point. `schemas.md` §3 already
defines ~16 trailers across universal, state, assignment, completion, and
error categories. Adding four more trailers under a "stack" category is the
smallest possible delta that gives the protocol first-class awareness of
stacks, because:

1. **The validation machinery already exists.** `trailer-validate.ts`
   reads trailers with `parseTrailersMulti` (line 5), walks a list of
   rules, emits named violations. Each new rule is a ~20-line function in
   the existing pattern.
2. **The state machinery already exists.** `lifecycle-check.ts` walks a
   branch's commits and enforces per-branch invariants. A new rule
   "`Stack-Id` and `Stack-Position` must be stable across a branch's
   lifecycle" slots in next to the existing post-ASSIGNED walk.
3. **The propagation machinery already exists.** `commit.ts` lines 66-72
   auto-inject `Agent-Id`, `Session-Id`, and `Heartbeat` on every commit.
   Adding a read-ASSIGNED-once, propagate-forever pattern for `Stack-Id`
   is a 20-line extension.
4. **The dispatch machinery already exists.** `dispatch.ts` already gates
   worker spawn on branch existence (lines 42-53). Adding a
   `Stack-Base`-is-COMPLETED gate is one extra `exec('git log …')` call
   before the worktree is created.
5. **The DAG machinery already exists.** `dag-check.ts` runs Kahn's
   algorithm over agents-and-dependencies. Adding a stack ordering
   constraint that says "agents sharing a `Stack-Id` must appear in
   ascending `Stack-Position`" is a single post-Kahn pass.

The cost is bounded: four new trailers, seven new validation rules, one
dispatch-time check, one DAG-time check, one `mcagent-spec.md` conformance
MUST, one paragraph in `protocol.md` §7. The payoff is that every sharp
edge the RFP lists in §1–§5 either disappears or collapses into a single
rule in a single validator.

### 2.3 The alternative is worse debt

The alternative is keeping the schema frozen and paying the same debt
forever in tooling. Every `gh stack view --json` invocation would have to
re-derive stack topology from branch-name regex, or from commit-body
conventions, or from a sidecar file that has to stay in lockstep with git.
The first time a worker needs to know its position, the answer has to be
computed from scratch. The first time a reviewer asks which `loom/*`
branch corresponds to a PR, the mapping is brittle.

Worse: debt-in-tooling drifts away from debt-in-protocol. A protocol debt
is paid once, documented in `schemas.md`, and enforced by a validator.
Tooling debt reappears every time a new tool is written. Team 3's angle
says: pay it once.

### 2.4 What we give up

- **A bigger schema.** Four new trailers is ~25% growth in the
  universal-plus-state trailer vocabulary. We accept this.
- **A larger reviewable diff.** The implementation PR touches five
  loom-tools files (`trailer-validate.ts`, `lifecycle-check.ts`,
  `commit.ts`, `dispatch.ts`, `dag-check.ts`), `schemas.md`,
  `protocol.md`, `mcagent-spec.md`, and the worker template. We accept
  this.
- **A small new sharp edge in stack-mode mid-stack rework.** A worker that
  forgets to propagate `Stack-Id` on a later commit gets rejected by
  `trailer-validate`, which is a loud, early failure — not a silent
  divergence. We accept this.

What we **do not** give up:

- Branch naming (`loom/<agent>-<slug>` is unchanged).
- The `ASSIGNED → PLANNING → IMPLEMENTING → COMPLETED` state machine.
- The `--no-ff` audit trail invariant.
- Worker scope enforcement.
- The "orchestrator-only writes" invariant on `loom/*` branches.
- Backward compatibility with non-stack epics.

---

## 3. What changes

This section enumerates every component the proposal modifies, with file
paths and (where useful) approximate insertion points.

### 3.1 New trailers in `schemas.md` §3 (trailer vocabulary)

Primary file:
`/home/willem/.claude/plugins/cache/loom-plugin/loom/0.1.0/skills/loom/references/schemas.md`
— add a new subsection **3.6 Stack trailers (stack-mode commits)** between
the existing §3.5 (error trailers) and §4 (required trailers per state):

| Trailer | Type | Description |
|---------|------|-------------|
| `Stack-Id` | UUID v4 | Stable identifier for the stack this commit belongs to. Identical across every commit on every branch in the stack. Required on ASSIGNED commits in stack-mode. Once observed on a branch, auto-injected on every subsequent commit by `commit.ts`. |
| `Stack-Position` | integer >= 1 | 1-indexed position of this branch within its stack, counted from the bottom (closest to trunk). Required on stack-mode ASSIGNED commits. Unique within a `Stack-Id`. |
| `Stack-Base` | `<agent>/<slug>` or `trunk` | Immediate downstack neighbor. Resolves to a LOOM branch via the existing `<agent>/<slug>` → `loom/<agent>-<slug>` mapping in §2. For position 1, value MUST be `trunk`. |
| `Stack-Epic` | kebab-case slug | Human-readable name of the epic the stack belongs to. Used by the dispatcher for logging and by the projector for commit-message generation. Stable across all members of one `Stack-Id`. |

Additionally, a new **completion trailer**:

| Trailer | Type | Description |
|---------|------|-------------|
| `Stack-Base-SHA` | git SHA | The integrator-observed SHA of the downstack neighbor (or trunk) at the moment this branch was integrated. Recorded by the orchestrator on the COMPLETED commit, or on the post-terminal `chore(loom):` integration commit. Used by rebase reconciliation (§5) to detect stale bases. |

All four stack trailers and `Stack-Base-SHA` are **conditional on
stack-mode**. A non-stack epic never sees them. A legacy validator that
does not know about them ignores them as unknown trailers.

### 3.2 New required-trailer column in `schemas.md` §4

Add a "Stack-mode" modifier column to §4.1 (ASSIGNED) and §4.3 (COMPLETED):

§4.1 ASSIGNED (stack-mode additions):

| Trailer | Required (non-stack) | Required (stack-mode) |
|---------|----------------------|-----------------------|
| `Stack-Id` | no | yes |
| `Stack-Position` | no | yes |
| `Stack-Base` | no | yes |
| `Stack-Epic` | no | yes |

§4.3 COMPLETED (stack-mode additions):

| Trailer | Required (non-stack) | Required (stack-mode) |
|---------|----------------------|-----------------------|
| `Stack-Id` | no | yes (auto-inherited) |
| `Stack-Position` | no | yes (auto-inherited) |

`Stack-Base-SHA` is written by the orchestrator at integration time and
therefore lives on the integration commit (or the terminal COMPLETED
commit if integration is in-place), not on a worker commit. It is not
worker-authored. See §6 below.

### 3.3 New validation rules in `schemas.md` §7

Add seven new rules to §7.1 and §7.2 (immediately after the existing
rule 14):

- **15. `stack-id-format`** — `Stack-Id`, if present, MUST be UUID v4
  (the same pattern `Session-Id` uses per rule 3).
- **16. `stack-position-positive`** — `Stack-Position`, if present, MUST
  be a decimal integer >= 1.
- **17. `stack-position-unique`** — within the set of unintegrated
  branches sharing a `Stack-Id`, each `Stack-Position` value MUST appear
  on at most one branch. Enforced at dispatch time and at integration
  time.
- **18. `stack-base-resolvable`** — `Stack-Base`, if present, either
  equals the literal string `trunk` or resolves to an existing `loom/*`
  branch (via the `<agent>/<slug>` → `loom/<agent>-<slug>` mapping) that
  carries the same `Stack-Id`.
- **19. `stack-base-position`** — `Stack-Base`'s `Stack-Position` MUST
  equal this branch's `Stack-Position` minus 1; or `Stack-Base` is
  `trunk` and this branch's `Stack-Position` is 1.
- **20. `stack-trailers-inheritance`** — once a branch has observed a
  `Stack-Id` on its ASSIGNED commit, every subsequent commit on the
  branch MUST carry the same `Stack-Id` and the same `Stack-Position`.
  Mirrors the way `Session-Id` propagates but for stack identity.
- **21. `stack-dependencies-consistency`** — if an ASSIGNED commit
  declares `Stack-Base: <agent>/<slug>` (non-trunk), then its
  `Dependencies` trailer MUST include that exact ref. Eliminates the
  "two ways to say the same thing" footgun by making the stack ordering
  a strengthened version of the dependency ordering.

### 3.4 `trailer-validate.ts` changes

File:
`/home/willem/bitswell/bitswell/repos/bitswell/loom-tools/src/tools/trailer-validate.ts`

The existing tool already handles universal and per-state validation in
a single pass. Changes land in the ASSIGNED block (lines 213-247) and
after the `Scope-Expand` check (lines 279-305).

**A. Constants and regex** (above the existing `HEARTBEAT_RE` on line 51):

```ts
const UUID_V4_RE =
  /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
const STACK_BASE_RE = /^(trunk|[a-z0-9]+(-[a-z0-9]+)*\/[a-z0-9]+(-[a-z0-9]+)*)$/;
const STACK_EPIC_RE = /^[a-z0-9]+(-[a-z0-9]+)*$/;
```

**B. Stack-mode detection** in the ASSIGNED branch (line ~213):

```ts
if (taskStatus === 'ASSIGNED') {
  // ...existing Assigned-To / Assignment / Scope / Dependencies / Budget checks...

  const stackId = first(trailers, 'Stack-Id');
  const stackMode = stackId !== undefined;
  if (stackMode) {
    for (const [trailer, rule] of [
      ['Stack-Position', 'stack-position-required'],
      ['Stack-Base', 'stack-base-required'],
      ['Stack-Epic', 'stack-epic-required'],
    ] as const) {
      if (!first(trailers, trailer)) {
        violations.push({
          rule,
          detail: `stack-mode ASSIGNED commit must have a ${trailer} trailer`,
          severity: 'error',
        });
      }
    }
  }
}
```

**C. Format rules** (after line 305, alongside `Scope-Expand` format
checks — these fire on every commit, not only ASSIGNED, so
`stack-trailers-inheritance` on later commits also gets the format guarantees):

```ts
const stackId = first(trailers, 'Stack-Id');
if (stackId !== undefined && !UUID_V4_RE.test(stackId)) {
  violations.push({
    rule: 'stack-id-format',
    detail: `Stack-Id '${stackId}' is not a valid UUID v4`,
    severity: 'error',
  });
}

const stackPosition = first(trailers, 'Stack-Position');
if (stackPosition !== undefined) {
  if (!/^\d+$/.test(stackPosition) || parseInt(stackPosition, 10) < 1) {
    violations.push({
      rule: 'stack-position-positive',
      detail: `Stack-Position '${stackPosition}' must be a positive integer`,
      severity: 'error',
    });
  }
}

const stackBase = first(trailers, 'Stack-Base');
if (stackBase !== undefined && !STACK_BASE_RE.test(stackBase)) {
  violations.push({
    rule: 'stack-base-format',
    detail: `Stack-Base '${stackBase}' must be 'trunk' or '<agent>/<slug>'`,
    severity: 'error',
  });
}

const stackEpic = first(trailers, 'Stack-Epic');
if (stackEpic !== undefined && !STACK_EPIC_RE.test(stackEpic)) {
  violations.push({
    rule: 'stack-epic-format',
    detail: `Stack-Epic '${stackEpic}' must be kebab-case`,
    severity: 'error',
  });
}
```

`trailer-validate.ts` does **not** enforce `stack-position-unique`,
`stack-base-resolvable`, `stack-base-position`, or
`stack-dependencies-consistency` — those are multi-commit invariants
that live in `lifecycle-check.ts` and `dispatch.ts` respectively. This
preserves the existing division of labor: `trailer-validate` is a pure
per-commit check.

The `TASK_STATUS_ENUM` on line 34 does NOT change. Stack-mode is
orthogonal to lifecycle state; it is a modifier, not a new state.

### 3.5 `lifecycle-check.ts` changes

File:
`/home/willem/bitswell/bitswell/repos/bitswell/loom-tools/src/tools/lifecycle-check.ts`

Three new branch-level rules, added after the existing post-walk invariant
block (in the same region as the single-state terminal check):

- **`lifecycle-stack-inheritance`** — after walking `base..branch` and
  observing the ASSIGNED commit's `Stack-Id`, assert that every subsequent
  commit on the branch carries the same `Stack-Id`. Any commit that
  silently drops or changes `Stack-Id` is a violation. This catches the
  class of bugs where a worker bypasses `commit.ts`'s auto-injection (e.g.
  by shelling out to `git commit` directly) and omits the trailer.
- **`lifecycle-stack-position-stable`** — same walk, same check but for
  `Stack-Position`. A worker MUST NOT change its `Stack-Position`
  mid-lifecycle. Only the orchestrator changes position, and only on a
  fresh ASSIGNED commit.
- **`stack-base-stale`** — resolves `Stack-Base` to a branch, reads its
  current HEAD SHA, compares to this branch's `Stack-Base-SHA` trailer
  on its terminal or integration commit. A mismatch means the downstack
  layer has been reworked since this layer was integrated, and this layer
  needs an upstack rebase replay. This fires only after COMPLETED+integrated.

In code terms, add a new helper after the existing transition walker:

```ts
function checkStackInvariants(
  commits: CommitView[],
  strict: boolean,
): Violation[] {
  const v: Violation[] = [];
  let expectedStackId: string | undefined;
  let expectedStackPosition: string | undefined;
  for (const c of commits) {
    const sid = c.trailers['Stack-Id']?.[0];
    const spos = c.trailers['Stack-Position']?.[0];
    if (c.taskStatus === 'ASSIGNED' && sid !== undefined) {
      expectedStackId = sid;
      expectedStackPosition = spos;
      continue;
    }
    if (expectedStackId === undefined) continue; // non-stack branch
    if (sid === undefined) {
      v.push({
        rule: 'lifecycle-stack-inheritance',
        detail: `commit ${c.sha} on stack-mode branch is missing Stack-Id`,
        severity: 'error',
      });
    } else if (sid !== expectedStackId) {
      v.push({
        rule: 'lifecycle-stack-inheritance',
        detail: `commit ${c.sha} Stack-Id '${sid}' does not match ASSIGNED '${expectedStackId}'`,
        severity: 'error',
      });
    }
    if (spos !== undefined && spos !== expectedStackPosition) {
      v.push({
        rule: 'lifecycle-stack-position-stable',
        detail: `commit ${c.sha} Stack-Position '${spos}' does not match ASSIGNED '${expectedStackPosition}'`,
        severity: 'error',
      });
    }
  }
  return v;
}
```

The existing `TERMINAL_STATES` set (lines 36-39) does NOT change. The
`LEGAL` state-machine table (lines 47-60) does NOT change. Stack-mode is
strictly orthogonal to lifecycle states.

One semantic extension: post-terminal `chore(loom)` commits on a stack
branch MUST preserve `Stack-Id`. This is enforced by the same
`lifecycle-stack-inheritance` walker — it applies to every commit after
ASSIGNED regardless of whether it has a `Task-Status` trailer.

### 3.6 `dispatch.ts` changes

File:
`/home/willem/bitswell/bitswell/repos/bitswell/loom-tools/src/tools/dispatch.ts`

The existing tool is minimal: verify branch exists (lines 42-53), create
worktree (lines 55-72), return path and metadata (lines 74-79). The
stack-aware changes insert a single block between the branch verification
and the worktree creation — around line 54.

**A. Read the ASSIGNED commit's stack trailers** (new helper function,
above `handler`):

```ts
async function readStackContext(
  cwd: string,
  branch: string,
): Promise<{
  stackId?: string;
  stackPosition?: number;
  stackBase?: string;
  stackEpic?: string;
}> {
  const log = await exec(
    'git',
    [
      'log',
      '--grep=Task-Status: ASSIGNED',
      '-1',
      '--format=%(trailers)',
      branch,
    ],
    cwd,
  );
  if (log.exitCode !== 0) return {};
  const t = parseTrailersMulti(log.stdout);
  const stackId = t['Stack-Id']?.[0];
  if (!stackId) return {};
  const stackPosition = parseInt(t['Stack-Position']?.[0] ?? '', 10);
  return {
    stackId,
    stackPosition: isNaN(stackPosition) ? undefined : stackPosition,
    stackBase: t['Stack-Base']?.[0],
    stackEpic: t['Stack-Epic']?.[0],
  };
}
```

**B. Gate dispatch on `Stack-Base` readiness** (after the branch verify,
before the worktree add):

```ts
const stack = await readStackContext(cwd, branch);
if (stack.stackId && stack.stackBase && stack.stackBase !== 'trunk') {
  const baseBranch = `loom/${stack.stackBase.replace('/', '-')}`;
  const baseStatus = await exec(
    'git',
    [
      'log',
      '-1',
      '--format=%(trailers:key=Task-Status,valueonly)',
      '--grep=Task-Status:',
      baseBranch,
    ],
    cwd,
  );
  const status = baseStatus.stdout.trim();
  if (status !== 'COMPLETED') {
    return err(
      'stack-base-not-ready',
      `Stack-Base '${stack.stackBase}' is at status '${status || 'unknown'}', not COMPLETED`,
      true,
    );
  }
}
```

**C. Uniqueness check across unintegrated commits** (after stack
readiness):

```ts
if (stack.stackId && stack.stackPosition) {
  const siblings = await exec(
    'git',
    [
      'log',
      '--all',
      '--grep=Task-Status: ASSIGNED',
      `--grep=Stack-Id: ${stack.stackId}`,
      '--all-match',
      '--format=%H %(trailers:key=Stack-Position,valueonly)',
    ],
    cwd,
  );
  // parse lines, count positions equal to stack.stackPosition
  // if more than this branch's own ASSIGNED commit has the same
  // position, refuse dispatch with 'stack-position-unique' violation
}
```

**D. Emit the `STACK_*` env var block** in the dispatch result so the
runtime adapter can forward it to the worker process:

```ts
return ok({
  worktreePath: input.worktreePath,
  branch,
  agentId: input.agentId,
  phase: input.phase,
  env: stack.stackId
    ? {
        STACK_ID: stack.stackId,
        STACK_POSITION: String(stack.stackPosition),
        STACK_BASE_BRANCH:
          stack.stackBase === 'trunk'
            ? 'main'
            : `loom/${stack.stackBase?.replace('/', '-')}`,
        STACK_EPIC: stack.stackEpic ?? '',
      }
    : {},
});
```

This requires a small extension to `DispatchOutput`:

```ts
const DispatchOutput = z.object({
  worktreePath: z.string(),
  branch: z.string(),
  agentId: z.string(),
  phase: z.string(),
  env: z.record(z.string()).optional(),
});
```

The runtime adapter (mcagent) reads this `env` field and sets the
corresponding environment variables on the spawned worker process. A
non-stack dispatch returns `env: {}` and is byte-identical to today's
output.

### 3.7 `commit.ts` changes

File:
`/home/willem/bitswell/bitswell/repos/bitswell/loom-tools/src/tools/commit.ts`

The existing trailer auto-injection block is lines 66-72:

```ts
const allTrailers: Record<string, string> = {
  'Agent-Id': ctx.agentId,
  'Session-Id': ctx.sessionId,
  Heartbeat: new Date().toISOString(),
  ...(input.trailers ?? {}),
};
```

Extension: before constructing `allTrailers`, read the ASSIGNED commit's
stack context from the current branch. If present, fold
`Stack-Id` and `Stack-Position` into the auto-inject set:

```ts
async function readBranchStackContext(
  cwd: string,
  branch: string,
): Promise<{ stackId?: string; stackPosition?: string }> {
  const log = await exec(
    'git',
    [
      'log',
      '--grep=Task-Status: ASSIGNED',
      '-1',
      '--format=%(trailers)',
      branch,
    ],
    cwd,
  );
  if (log.exitCode !== 0) return {};
  const t = parseTrailersMulti(log.stdout);
  return {
    stackId: t['Stack-Id']?.[0],
    stackPosition: t['Stack-Position']?.[0],
  };
}

// ...inside handler...
const stack = await readBranchStackContext(cwd, ctx.branch);
const allTrailers: Record<string, string> = {
  'Agent-Id': ctx.agentId,
  'Session-Id': ctx.sessionId,
  Heartbeat: new Date().toISOString(),
  ...(stack.stackId ? { 'Stack-Id': stack.stackId } : {}),
  ...(stack.stackPosition ? { 'Stack-Position': stack.stackPosition } : {}),
  ...(input.trailers ?? {}),
};
```

Caller-supplied trailers take precedence, so the orchestrator can still
explicitly set `Stack-Id` on a fresh ASSIGNED commit without a
read-back loop. Workers that never set `Stack-Id` directly get the
correct value auto-injected and never have to think about it. This is
the same contract `Session-Id` already enjoys.

The commit's `Scope-Expand` / `Scope` validation logic is unchanged.
`validateScope` (line 43) is unchanged — stack-mode does not relax scope.

### 3.8 `dag-check.ts` changes

File:
`/home/willem/bitswell/bitswell/repos/bitswell/loom-tools/src/tools/dag-check.ts`

The existing `AgentEntry` schema (lines 5-10):

```ts
const AgentEntry = z.object({
  id: z.string(),
  dependencies: z.array(z.string()),
});
```

Extension: add optional stack fields.

```ts
const AgentEntry = z.object({
  id: z.string(),
  dependencies: z.array(z.string()),
  stackId: z.string().optional(),
  stackPosition: z.number().int().positive().optional(),
});
```

And a new violation rule:

```ts
const Violation = z.object({
  rule: z.enum([
    'dag-cycle',
    'dag-missing-dep',
    'dag-self-dep',
    'dag-stack-order-violation',
  ]),
  detail: z.string(),
});
```

After Kahn's algorithm produces `integrationOrder`, add a post-pass:

```ts
// Verify agents sharing a Stack-Id appear in ascending Stack-Position.
const byStack = new Map<string, { id: string; pos: number }[]>();
for (const a of input.agents) {
  if (!a.stackId || a.stackPosition === undefined) continue;
  if (!byStack.has(a.stackId)) byStack.set(a.stackId, []);
  byStack.get(a.stackId)!.push({ id: a.id, pos: a.stackPosition });
}
for (const [sid, members] of byStack) {
  const indices = members.map((m) => ({
    m,
    idx: integrationOrder.indexOf(m.id),
  }));
  indices.sort((a, b) => a.idx - b.idx);
  for (let i = 1; i < indices.length; i++) {
    if (indices[i - 1].m.pos >= indices[i].m.pos) {
      violations.push({
        rule: 'dag-stack-order-violation',
        detail: `Stack-Id ${sid}: '${indices[i - 1].m.id}' (position ${indices[i - 1].m.pos}) integrates before '${indices[i].m.id}' (position ${indices[i].m.pos})`,
      });
    }
  }
}
```

Cycle detection, missing-dep detection, and self-dep detection are
unchanged. Stacks are a strictly stronger ordering constraint on top of
the DAG.

### 3.9 `mcagent-spec.md` conformance rule

File:
`/home/willem/.claude/plugins/cache/loom-plugin/loom/0.1.0/skills/loom/references/mcagent-spec.md`

Add one new MUST to the runtime conformance section:

> **R.19 (Stack env propagation).** A conforming agent runtime MUST, when
> dispatching a worker for a stack-mode assignment, read the ASSIGNED
> commit's `Stack-Id`, `Stack-Position`, `Stack-Base`, and `Stack-Epic`
> trailers, set the corresponding `STACK_ID`, `STACK_POSITION`,
> `STACK_BASE_BRANCH`, and `STACK_EPIC` environment variables on the
> worker process, and MUST NOT silently strip stack trailers from
> derivative commits written via the commit tool.

### 3.10 `protocol.md` §7 (coordination)

File:
`/home/willem/.claude/plugins/cache/loom-plugin/loom/0.1.0/skills/loom/references/protocol.md`

Add one new paragraph to §7 (coordination invariants):

> **Stack coordination.** If an assignment commit declares
> `Stack-Base: <agent>/<slug>`, that ref MUST also appear in the
> commit's `Dependencies` trailer. The stack is the contract; the
> dependency graph is a consequence. A validator that sees one without
> the other MUST emit `stack-dependencies-consistency`. The orchestrator
> is the sole authority that mints `Stack-Id` values; workers propagate
> `Stack-Id` and `Stack-Position` verbatim and MUST NOT author
> derivative commits that change either.

And one new paragraph to §3.3 (integrate):

> **Stack base recording.** When integrating a stack-mode branch, the
> orchestrator MUST record `Stack-Base-SHA: <sha>` on the integration
> commit (or on the terminal COMPLETED commit for in-place integration),
> where `<sha>` is the HEAD of the `Stack-Base` branch (or the HEAD of
> trunk, for position 1) at the moment of integration. This SHA is the
> audit-trail equivalent of `gh-stack`'s rebase pointer, and is how
> `lifecycle-check`'s `stack-base-stale` rule detects when an upstack
> layer needs a rebase replay.

### 3.11 Worker template changes

File: the `loom` skill's worker-template reference. The template gets
four new environment variables documented as inputs to stack-mode
workers:

- `STACK_ID` — present iff stack-mode. UUID v4.
- `STACK_POSITION` — 1-indexed integer.
- `STACK_BASE_BRANCH` — the fully-qualified `loom/*` branch of the
  downstack neighbor, or `main` for position 1.
- `STACK_EPIC` — kebab-case epic slug.

The template gains a "**If you are a stack-mode worker**" sidebar that
tells the worker:

1. You MAY `git show STACK_BASE_BRANCH:<path>` to read files in your
   downstack layer. Do not copy them into your own scope; reference them.
2. You MUST NOT emit commits that change `Stack-Id` or `Stack-Position`.
   The `commit` tool auto-injects these; do not override them.
3. If you need a change in the layer below you, you cannot make it.
   Commit `BLOCKED` with a `Blocked-Reason` explaining the downstack
   change needed. The orchestrator will spawn a new ASSIGNED branch at
   the correct position.

### 3.12 Zero changes

The following components do NOT change:

- **Branch naming** (`loom/<agent>-<slug>`). Branch names carry agent
  and assignment; they do not carry topology.
- **The lifecycle state machine** in `lifecycle-check.ts` lines 47-60.
  Stack-mode is orthogonal to `ASSIGNED → PLANNING → IMPLEMENTING →
  COMPLETED`.
- **Scope enforcement** (`validateScope` in `commit.ts` line 43). Each
  layer's worker writes only its own scope. Stack-mode does not grant
  cross-layer write access.
- **The `Session-Id` model.** Each agent invocation still mints its own
  session; resuming a BLOCKED branch still gets a new session.
- **PR authority.** Only the orchestrator creates PRs. Workers never run
  `gh pr create` or `gh stack submit`.
- **The `--no-ff` audit trail.** `loom/*` branches are never rebased.
  See §5 for how this co-exists with `gh-stack`'s rebase model.

---

## 4. Branch naming and scope

### 4.1 Branch names are agent-and-assignment, not topology

The key insight of this proposal is that LOOM's branch namespace
(`loom/<agent>-<slug>`) already carries two facts: **who** is working on
something (`<agent>`) and **what** they are working on (`<slug>`).
Overloading it with a third fact — **which layer** of a stack they are
working on — would force a branch rename on every rework. A rename
breaks:

- The 1:1 mapping between assignment slugs and `loom/*` branches that
  `schemas.md` §2 relies on.
- The dispatch tool's branch-verify step (`dispatch.ts` lines 42-53).
- Every validator that parses branch names to extract agent/slug.
- The `@<agent>` mention UX in orchestrator prompts.

Instead, stack layer is a **trailer** on the ASSIGNED commit, read by
`dispatch.ts` and auto-injected into every subsequent commit by
`commit.ts`. The branch name tells you who's doing the work; the
trailer tells you where in the stack that work sits. These are two
independent axes and deserve two independent encodings.

### 4.2 Scope is unchanged

A worker's `Scope` trailer is exactly what it is today. Each layer in a
stack has its own scope, declared at ASSIGNED time by the orchestrator,
enforced at commit time by `validateScope` in `commit.ts`. A stack-mode
worker CANNOT use `Scope-Expand` to reach into a downstack layer's
files. That would require a new assignment at the correct layer, exactly
as the `gh-stack` skill recommends in its "navigate down the stack"
rule (SKILL.md agent rule 9).

Concretely, if `ratchet/auth-middleware` (position 1) declares
`Scope: pkg/auth/**` and `moss/api-endpoints` (position 2) declares
`Scope: pkg/api/**`, moss cannot use `Scope-Expand: pkg/auth/** -- need
to fix a bug` to write into the auth layer. The orchestrator instead
spawns a new ASSIGNED branch (e.g., `loom/ratchet-auth-middleware-fix`)
at position 1's layer with a fresh `Session-Id`, and the fix flows up
through rebase replay (see §5).

### 4.3 Cross-layer read visibility

One small but documented extension of worker authority: a stack-mode
worker MAY read the tree of its `Stack-Base` branch during its
IMPLEMENTING phase. This is already legal today via `git show
<branch>:<path>` in a cross-worktree repo; the proposal just makes it a
documented contract instead of an accident.

The `STACK_BASE_BRANCH` env var is the pointer that makes "which branch
is my base" a protocol fact instead of a filesystem guess. A worker
that needs to know the type signature of a function it is calling in
the downstack layer runs:

```bash
git show "$STACK_BASE_BRANCH:pkg/auth/middleware.go" | head -40
```

This is read-only. The worker NEVER writes to `$STACK_BASE_BRANCH`.
Writes are rejected by `validateScope` at commit time because the base
branch's files are outside the worker's `Scope`.

### 4.4 New dispatch-time failure mode

A new class of error appears at dispatch: two ASSIGNED branches claim
the same `Stack-Id` + `Stack-Position`. Today this would silently
collide in a projection layer (or corrupt a sidecar file). With this
proposal, `dispatch.ts` refuses to spawn either worker and emits a
`stack-position-unique` violation. The orchestrator resolves by
retiring one of the two ASSIGNED commits (see §7 on the
`Stack-Supersedes` follow-up) and re-dispatching.

This is a **better** error story than silent collision. The validator
catches it at the earliest possible moment, names it, and gives the
orchestrator a concrete remediation path.

### 4.5 `Scope-Expand` interaction

The existing `Scope-Expand` trailer (`trailer-validate.ts` lines
279-305) continues to work within a layer. A worker that discovers it
needs to touch a sibling file inside its own layer's scope can still
request expansion.

A new rule `scope-expand-cross-stack` rejects `Scope-Expand` paths that
overlap any downstack layer's integrated changes for the same
`Stack-Id`. This is a strict-mode check (`strict: true` in
`TrailerValidateInput` line 13); non-strict mode emits a warning. The
rule prevents a stack-mode worker from accidentally bypassing scope by
expanding into territory the downstack layer already owns.

---

## 5. Merge vs rebase

This is the hardest reconciliation in the RFP. `gh-stack` is built around
rebase-and-force-push. LOOM is built around `--no-ff` merges that
preserve the full audit trail. Team 3's answer: **keep both, in two
different namespaces, with a one-way projection**.

### 5.1 The `loom/*` namespace: merge-only, audit-preserving

Integration of a `loom/*` branch is unchanged from today. The
orchestrator runs:

```bash
git merge --no-ff --log loom/ratchet-auth-middleware \
  -m "merge(loom): integrate auth-middleware layer 1"
```

The merge commit carries:

- `Agent-Id: bitswell`
- `Session-Id: <bitswell session>`
- (optional) `Stack-Base-SHA: <sha of trunk at integration>`

The `--no-ff` flag preserves the audit trail exactly as today. No
history is rewritten on `loom/*`. Every commit ever written by a worker
remains reachable from trunk through the merge commit.

### 5.2 The `stack/*` namespace: disposable projection

After integration, a thin **read-only projector** walks `Stack-Id`
groups, picks the latest COMPLETED branch per `Stack-Position`,
cherry-picks each layer's integration range onto a disjoint
`stack/<epic-slug>/<NN>-<slug>` branch namespace, and feeds that
namespace to `gh stack init --adopt` and `gh stack submit --auto
--draft`.

Concretely, the projector:

1. Runs `git log --all --grep='Stack-Id: <id>'
   --format='%H%n%(trailers)'` to enumerate members.
2. Groups by `Stack-Position`.
3. For each position, selects the most recent COMPLETED branch.
4. Cherry-picks the layer's commits onto
   `stack/<epic>/<NN>-<slug>`.
5. Calls `gh stack init --adopt stack/<epic>/01-... stack/<epic>/02-...
   ...` and `gh stack submit --auto --draft`.

The `stack/*` namespace is **explicitly disposable**. It exists only to
feed `gh-stack`. `gh-stack` is allowed to rebase and force-push it
freely. The LOOM audit trail lives in the `loom/*` namespace and is
never touched. This is exactly Team 1's publisher approach — the
difference is that the topology the projector publishes comes from
**trailers**, not from filesystem conventions or re-derived guesses.

This is the resolution of the "gh-stack uses rebase + force-push, LOOM
uses `--no-ff`" objection. Both are true, and they apply to two
different namespaces. The `loom/*` namespace preserves the audit trail.
The `stack/*` namespace is a projection that `gh-stack` owns.

### 5.3 The `Stack-Base-SHA` pivot

`Stack-Base-SHA` is the bridge that makes mid-stack rework detectable.
When layer `N` is integrated, the orchestrator records the HEAD SHA of
layer `N-1` at that instant. Later, when layer `N-1` is amended (e.g.,
a reviewer rejects the auth middleware and a new ASSIGNED branch lands
a fix), layer `N`'s `Stack-Base-SHA` now refers to a stale commit.

`lifecycle-check.ts`'s new `stack-base-stale` rule walks the integration
log, compares each integrated layer's `Stack-Base-SHA` to the current
HEAD of its `Stack-Base` branch, and flags any layer whose base has
moved. This gives the orchestrator a precise list of "which layers need
a rebase replay".

### 5.4 Rework reconciliation as a fresh assignment

When `stack-base-stale` fires, the orchestrator spawns a **new** ASSIGNED
commit at the affected layer:

- New branch: e.g., `loom/moss-api-endpoints-v2`.
- New `Session-Id` (fresh invocation).
- Same `Stack-Id` (same stack).
- Same `Stack-Position` (same layer).
- Updated `Stack-Base` pointing at the new bottom.
- A new body line: `Replaces: loom/moss-api-endpoints`.

The old branch stays in history as the record of the pre-rework state.
The new branch is the active layer for that position. The `stack/*`
projector picks the latest COMPLETED branch per position, so on the
next projection it picks the new branch automatically.

Note that `stack-position-unique` at dispatch time needs a small
refinement to handle this: it must allow a new ASSIGNED commit at
position `P` if the previous branch at position `P` is already
COMPLETED and integrated. The check is: "among *unintegrated and
non-superseded* ASSIGNED branches sharing a `Stack-Id`, each
`Stack-Position` appears at most once". This is exactly the
`Stack-Supersedes` follow-up described in §7.

### 5.5 Why a projection layer beats in-place rebase

The RFP could reasonably ask: why not just let the orchestrator rebase
`loom/*` branches directly? The answer is that rebasing `loom/*`
branches breaks three invariants simultaneously:

- `lifecycle-check.ts`'s terminal-state rule (rule 12 in `schemas.md`
  §7) would need to be relaxed to allow post-terminal commits to be
  rewritten, which means every lifecycle guarantee is weaker.
- `Session-Id` continuity across a rebased branch becomes ambiguous.
  The rebased commits are different SHAs with the same logical content;
  downstream validators that join on SHA have to change.
- Key findings and decisions recorded in commit trailers on a rebased
  branch become harder to cite. A reviewer who says "see the decision
  on <sha>" now has a stale reference.

All three of these are **real** costs. The projection-to-`stack/*`
approach pays none of them. The `loom/*` namespace is immutable; the
`stack/*` namespace is disposable. Audit truth lives in the namespace
that is never rewritten.

### 5.6 What the projector does NOT do

- It does NOT write `loom/*` branches. Ever.
- It does NOT mutate commit trailers. It is read-only on the LOOM side.
- It does NOT call `gh pr create` directly. It calls `gh stack submit
  --auto --draft`, which is the `gh-stack`-mediated PR creation path.
- It does NOT run on workers. It runs inside the orchestrator as a new
  `stack-project` recipe, invoked by bitswell when an epic is ready to
  ship.

---

## 6. Worker authority

The RFP asks which LOOM invariants this proposal preserves, relaxes, or
breaks. The answer is that **no existing invariant is relaxed or broken;
one is added**.

### 6.1 Invariants preserved verbatim

- **Workspace-write monopoly.** Only the orchestrator (bitswell) can
  write to `loom/*` branches outside of a worker's active session.
  Stack-mode does not change this.
- **Scope enforcement at commit time.** `validateScope` in `commit.ts`
  line 43 is unchanged. Each worker writes only its own scope.
- **No cross-agent branch writes.** Workers never push to another
  agent's branch. Stack-mode does not change this.
- **PR authority.** Only the orchestrator creates PRs. Workers never
  run `gh pr create` or `gh stack submit`. Stack-mode does not change
  this.
- **`Session-Id` per-invocation uniqueness.** Each agent invocation
  mints a fresh `Session-Id`. Stack-mode does not change this.
- **The lifecycle state machine.** `ASSIGNED → PLANNING → IMPLEMENTING
  → COMPLETED`, with `BLOCKED`/`FAILED` off-ramps, is unchanged.
  Stack-mode is orthogonal to state.
- **The `--no-ff` audit trail.** `loom/*` branches are never rebased.
  The projection lives in `stack/*`, which is disposable.

### 6.2 Invariants added

- **Stack-identity monopoly.** Only the orchestrator mints `Stack-Id`
  values (in the initial ASSIGNED commit). Workers propagate them
  verbatim via `commit.ts` auto-injection. A worker that tries to
  override `Stack-Id` or `Stack-Position` in a later commit is rejected
  by `trailer-validate`'s new `stack-trailers-inheritance` rule.
- **Stack-base stability.** A worker's `Stack-Base` does not change
  mid-lifecycle. A layer's base is frozen at ASSIGNED time. If the
  base needs to change, the orchestrator spawns a new ASSIGNED commit
  (new branch, new `Session-Id`).
- **Dispatch-time base-ready gate.** A stack-mode worker cannot be
  dispatched until its `Stack-Base` has reached COMPLETED. This is a
  strictly stronger version of the existing `Dependencies` gate, not a
  relaxation.

### 6.3 One documented extension

A stack-mode worker MAY read the tree of its `Stack-Base` branch via
`git show $STACK_BASE_BRANCH:<path>`. This is read-only. It is already
legal today via cross-worktree git reads; the proposal just makes it a
documented part of the worker contract instead of an accident.

Writes to the base branch remain forbidden by `validateScope` at commit
time. There is no way for a stack-mode worker to reach through the
`Stack-Base` pointer into the filesystem and write.

### 6.4 Workers never invoke `gh stack`

This is absolute. Workers do not run `gh stack init`, `gh stack add`,
`gh stack submit`, `gh stack rebase`, `gh stack sync`, or any other
`gh stack` command. The `gh-stack` SKILL.md is not loaded into worker
sessions. Workers do not know `gh-stack` exists; they only know they
have `STACK_*` env vars and a documented contract about how to read
them.

`gh-stack` is invoked exclusively by the orchestrator's `stack-project`
recipe at integration time. The recipe runs inside bitswell's session,
not inside a worker's. This preserves the LOOM invariant "only the
orchestrator manipulates remote refs" verbatim.

### 6.5 Explicit invariant matrix

| Invariant | Today | With Team 3's proposal |
|-----------|-------|------------------------|
| Only orchestrator writes `loom/*` | yes | yes |
| Workers enforce `Scope` at commit | yes | yes |
| `--no-ff` merges on integration | yes | yes |
| Branch names are `loom/<agent>-<slug>` | yes | yes |
| `Session-Id` uniqueness | yes | yes |
| State machine `ASSIGNED → ... → COMPLETED` | yes | yes |
| Only orchestrator creates PRs | yes | yes |
| **Stack identity is orchestrator-owned** | n/a | yes (new) |
| **Stack-base-ready dispatch gate** | n/a | yes (new) |
| **Stack-mode workers read `STACK_BASE_BRANCH`** | n/a | yes (new, read-only) |

Zero existing rows change. Three new rows are added. This is the
smallest possible footprint for a first-class stack primitive.

---

## 7. End-to-end example

This section traces a concrete 3-agent epic, `auth-stack`, from initial
orchestrator decomposition through merged `gh-stack` PRs. The stack is:

```
main (trunk)
 └── loom/ratchet-auth-middleware    (position 1, Stack-Base: trunk)
  └── loom/moss-api-endpoints        (position 2, Stack-Base: ratchet/auth-middleware)
   └── loom/ratchet-frontend         (position 3, Stack-Base: moss/api-endpoints)
```

### Phase 0 — Epic decomposition

The orchestrator (bitswell) decides issue #99 "add session-based auth"
is large enough to warrant a stack. It mints a fresh `Stack-Id`:

```
8f3b2e1a-4d6c-4e3b-9a1f-7c2d3e4f5a6b
```

And decomposes the epic into three layers, with one ASSIGNED commit per
layer. Each assignment commit is a `task(...)` commit on the
corresponding `loom/*` branch.

**Commit on `loom/ratchet-auth-middleware`:**

```
task(ratchet): implement session auth middleware

Build the auth middleware that validates session tokens on incoming
requests. Defines the Session type, token verification, and error
paths. First layer of the auth-stack epic.

Agent-Id: bitswell
Session-Id: 2d8f1a3c-6b4e-4a7d-b5c9-1e8f3a2b4c6d
Task-Status: ASSIGNED
Assigned-To: ratchet
Assignment: auth-middleware
Scope: pkg/auth/**
Dependencies: none
Budget: 40000
Stack-Id: 8f3b2e1a-4d6c-4e3b-9a1f-7c2d3e4f5a6b
Stack-Position: 1
Stack-Base: trunk
Stack-Epic: auth-stack
```

**Commit on `loom/moss-api-endpoints`:**

```
task(moss): add user endpoints using auth middleware

Build /api/users endpoints that rely on the auth middleware from
layer 1. Depends on the Session type and middleware signature.
Second layer of the auth-stack epic.

Agent-Id: bitswell
Session-Id: 2d8f1a3c-6b4e-4a7d-b5c9-1e8f3a2b4c6d
Task-Status: ASSIGNED
Assigned-To: moss
Assignment: api-endpoints
Scope: pkg/api/**
Dependencies: ratchet/auth-middleware
Budget: 40000
Stack-Id: 8f3b2e1a-4d6c-4e3b-9a1f-7c2d3e4f5a6b
Stack-Position: 2
Stack-Base: ratchet/auth-middleware
Stack-Epic: auth-stack
```

Note `Dependencies` includes `ratchet/auth-middleware` — this satisfies
rule 21 (`stack-dependencies-consistency`).

**Commit on `loom/ratchet-frontend`:**

```
task(ratchet): add login UI calling the user endpoints

Build the frontend login page and session UI. Depends on the
/api/users endpoints from layer 2. Third and final layer of the
auth-stack epic.

Agent-Id: bitswell
Session-Id: 2d8f1a3c-6b4e-4a7d-b5c9-1e8f3a2b4c6d
Task-Status: ASSIGNED
Assigned-To: ratchet
Assignment: frontend
Scope: pkg/ui/**, pkg/client/**
Dependencies: moss/api-endpoints
Budget: 40000
Stack-Id: 8f3b2e1a-4d6c-4e3b-9a1f-7c2d3e4f5a6b
Stack-Position: 3
Stack-Base: moss/api-endpoints
Stack-Epic: auth-stack
```

### Phase 1 — Dispatch gate

Orchestrator invokes `dispatch` for all three assignments in parallel.

- `dispatch.ts` reads `loom/ratchet-auth-middleware`'s ASSIGNED commit,
  observes `Stack-Base: trunk`, which is always "ready". It creates
  the worktree at
  `.loom/agents/ratchet/worktrees/bitswell_bitswell_auth-middleware/`
  and returns:

  ```json
  {
    "worktreePath": ".loom/agents/ratchet/worktrees/bitswell_bitswell_auth-middleware",
    "branch": "loom/ratchet-auth-middleware",
    "agentId": "ratchet",
    "phase": "implementation",
    "env": {
      "STACK_ID": "8f3b2e1a-4d6c-4e3b-9a1f-7c2d3e4f5a6b",
      "STACK_POSITION": "1",
      "STACK_BASE_BRANCH": "main",
      "STACK_EPIC": "auth-stack"
    }
  }
  ```

- `dispatch.ts` for `loom/moss-api-endpoints` reads `Stack-Base:
  ratchet/auth-middleware`, resolves it to `loom/ratchet-auth-middleware`,
  runs `git log -1 --format='%(trailers:key=Task-Status,valueonly)'
  --grep='Task-Status:' loom/ratchet-auth-middleware`, observes
  `ASSIGNED` (not `COMPLETED`), and returns
  `err('stack-base-not-ready', ...)`. The worker is not spawned.

- Same for `loom/ratchet-frontend`: base is `moss/api-endpoints`,
  status is `ASSIGNED`, dispatch refuses.

Only position 1 spawns.

### Phase 2 — Layer 1 work

Ratchet's session on `loom/ratchet-auth-middleware`:

**First commit (begin):**

```
chore(loom): begin auth-middleware implementation

Agent-Id: ratchet
Session-Id: 9a7f2e1b-5c4d-4f8a-b3e7-2d9f1a4b5c6e
Task-Status: PLANNING
Heartbeat: 2026-04-14T12:00:00Z
Stack-Id: 8f3b2e1a-4d6c-4e3b-9a1f-7c2d3e4f5a6b
Stack-Position: 1
```

Note `Stack-Id` and `Stack-Position` are auto-injected by `commit.ts`
reading the ASSIGNED commit's stack context. Ratchet never sets them
explicitly.

**Implementation commits** (three of them, each auto-inheriting stack
trailers):

```
feat(auth): add Session type and token verifier

Defines Session, sessionFromToken, and the error variants.

Agent-Id: ratchet
Session-Id: 9a7f2e1b-5c4d-4f8a-b3e7-2d9f1a4b5c6e
Task-Status: IMPLEMENTING
Heartbeat: 2026-04-14T12:14:00Z
Stack-Id: 8f3b2e1a-4d6c-4e3b-9a1f-7c2d3e4f5a6b
Stack-Position: 1
```

```
feat(auth): add middleware that applies the verifier

The middleware reads the Authorization header, calls sessionFromToken,
and injects Session into the request context.

Agent-Id: ratchet
Session-Id: 9a7f2e1b-5c4d-4f8a-b3e7-2d9f1a4b5c6e
Task-Status: IMPLEMENTING
Heartbeat: 2026-04-14T12:27:00Z
Stack-Id: 8f3b2e1a-4d6c-4e3b-9a1f-7c2d3e4f5a6b
Stack-Position: 1
```

```
test(auth): cover valid, expired, and malformed tokens

Three Rust tests under pkg/auth/tests/middleware_test.rs.

Agent-Id: ratchet
Session-Id: 9a7f2e1b-5c4d-4f8a-b3e7-2d9f1a4b5c6e
Task-Status: IMPLEMENTING
Heartbeat: 2026-04-14T12:41:00Z
Stack-Id: 8f3b2e1a-4d6c-4e3b-9a1f-7c2d3e4f5a6b
Stack-Position: 1
```

**Completion commit:**

```
feat(auth): complete session auth middleware

Layer 1 of auth-stack. Exports Session, sessionFromToken, and the
Middleware handler. All three tests green.

Agent-Id: ratchet
Session-Id: 9a7f2e1b-5c4d-4f8a-b3e7-2d9f1a4b5c6e
Task-Status: COMPLETED
Files-Changed: 5
Key-Finding: token TTL must be compared against server-side clock, not token issuer clock -- prevents replay during clock drift
Heartbeat: 2026-04-14T13:02:00Z
Stack-Id: 8f3b2e1a-4d6c-4e3b-9a1f-7c2d3e4f5a6b
Stack-Position: 1
```

**Orchestrator integration commit** (written by bitswell as part of the
merge):

```
merge(loom): integrate auth-middleware layer 1

Agent-Id: bitswell
Session-Id: 2d8f1a3c-6b4e-4a7d-b5c9-1e8f3a2b4c6d
Stack-Id: 8f3b2e1a-4d6c-4e3b-9a1f-7c2d3e4f5a6b
Stack-Position: 1
Stack-Base-SHA: f1e2d3c4b5a6978869fedcba0123456789abcdef
```

where `f1e2d3c4...` is trunk's HEAD at the moment of integration. Note
this is a post-terminal `chore(loom)`-class commit — no `Task-Status` —
but it DOES carry `Stack-Id` and `Stack-Position` per the
`lifecycle-stack-inheritance` rule, and it adds `Stack-Base-SHA` per
the new §3.3 integrate invariant.

### Phase 3 — Layer 2 dispatch

Now `loom/ratchet-auth-middleware` is COMPLETED and integrated.
Orchestrator re-runs `dispatch` for `loom/moss-api-endpoints`:

- Reads ASSIGNED commit, observes `Stack-Base: ratchet/auth-middleware`.
- Resolves to `loom/ratchet-auth-middleware`, runs status query, gets
  `COMPLETED`.
- Creates worktree at
  `.loom/agents/moss/worktrees/bitswell_bitswell_api-endpoints/`.
- Returns env:

  ```json
  {
    "STACK_ID": "8f3b2e1a-4d6c-4e3b-9a1f-7c2d3e4f5a6b",
    "STACK_POSITION": "2",
    "STACK_BASE_BRANCH": "loom/ratchet-auth-middleware",
    "STACK_EPIC": "auth-stack"
  }
  ```

Moss's worker starts. Its first move is to read the base layer's types
so it knows how to call the middleware:

```bash
git show "$STACK_BASE_BRANCH:pkg/auth/session.go" | head -30
git show "$STACK_BASE_BRANCH:pkg/auth/middleware.go" | head -30
```

This is legal — it's read-only and uses the env var set by the
runtime. Moss then writes its own files inside `pkg/api/**`. Scope is
enforced at commit time: any attempt to write `pkg/auth/**` is rejected
by `validateScope`.

Moss's ASSIGNED → PLANNING → IMPLEMENTING → COMPLETED cycle plays out
with stack trailers auto-inherited on every commit, mirroring layer 1.

**Representative implementing commit on `loom/moss-api-endpoints`:**

```
feat(api): add GET /api/users/me endpoint

Returns the current session's user. Wraps the endpoint with the
auth middleware from layer 1.

Agent-Id: moss
Session-Id: 3e4f5a6b-7c8d-4e9f-a0b1-c2d3e4f5a6b7
Task-Status: IMPLEMENTING
Heartbeat: 2026-04-14T14:12:00Z
Stack-Id: 8f3b2e1a-4d6c-4e3b-9a1f-7c2d3e4f5a6b
Stack-Position: 2
```

**Moss's COMPLETED commit:**

```
feat(api): complete user endpoints layer

Layer 2 of auth-stack. Exposes GET /api/users/me, GET /api/users/:id,
and PATCH /api/users/:id. All wrapped with auth middleware.

Agent-Id: moss
Session-Id: 3e4f5a6b-7c8d-4e9f-a0b1-c2d3e4f5a6b7
Task-Status: COMPLETED
Files-Changed: 7
Key-Finding: the middleware's Session context key collides with a stdlib constant in Go 1.24 -- had to rename to AuthSessionKey
Heartbeat: 2026-04-14T14:58:00Z
Stack-Id: 8f3b2e1a-4d6c-4e3b-9a1f-7c2d3e4f5a6b
Stack-Position: 2
```

**Moss's integration commit on trunk:**

```
merge(loom): integrate api-endpoints layer 2

Agent-Id: bitswell
Session-Id: 2d8f1a3c-6b4e-4a7d-b5c9-1e8f3a2b4c6d
Stack-Id: 8f3b2e1a-4d6c-4e3b-9a1f-7c2d3e4f5a6b
Stack-Position: 2
Stack-Base-SHA: 1234567890abcdef1234567890abcdef12345678
```

where `1234567890ab...` is `loom/ratchet-auth-middleware`'s HEAD at the
moment of integration — which equals the merge commit created in phase
2. This is the audit-trail equivalent of `gh-stack`'s rebase pointer.

### Phase 4 — Layer 3 dispatch and work

Same pattern. Ratchet's second session (fresh `Session-Id`, same
`Agent-Id: ratchet`) runs on `loom/ratchet-frontend` with
`STACK_BASE_BRANCH=loom/moss-api-endpoints`, reads moss's API types,
builds the UI inside `pkg/ui/**` and `pkg/client/**`, and commits
through to COMPLETED.

### Phase 5 — Mid-stack rework

A reviewer lands on layer 1 and finds a bug: `sessionFromToken`
accepts expired tokens if the exp claim is missing. Orchestrator's
response:

1. Creates a new ASSIGNED commit on a new branch
   `loom/moss-auth-middleware-fix` with the same `Stack-Id` and the
   same `Stack-Position: 1`:

   ```
   task(moss): fix missing-exp acceptance in auth middleware

   Reviewer found that sessionFromToken accepts tokens with no exp
   claim as valid. Fix the default to reject.

   Agent-Id: bitswell
   Session-Id: 4f5a6b7c-8d9e-4a0b-b1c2-d3e4f5a6b7c8
   Task-Status: ASSIGNED
   Assigned-To: moss
   Assignment: auth-middleware-fix
   Scope: pkg/auth/**
   Dependencies: none
   Budget: 20000
   Stack-Id: 8f3b2e1a-4d6c-4e3b-9a1f-7c2d3e4f5a6b
   Stack-Position: 1
   Stack-Base: trunk
   Stack-Epic: auth-stack
   Stack-Supersedes: loom/ratchet-auth-middleware
   ```

   The `Stack-Supersedes` trailer (a §7 follow-up) tells
   `dispatch.ts` that the uniqueness check should ignore the
   superseded branch. Without it, dispatch would refuse with
   `stack-position-unique`. See §7 below for why this is a
   follow-up rather than a core invariant.

2. Moss runs, fixes the bug, commits COMPLETED.

3. Orchestrator integrates `loom/moss-auth-middleware-fix` on trunk,
   records its `Stack-Base-SHA`.

4. `lifecycle-check.ts`'s `stack-base-stale` rule fires against
   `loom/moss-api-endpoints` and `loom/ratchet-frontend` — their
   integrated `Stack-Base-SHA` no longer matches the new HEAD of the
   base layer.

5. Orchestrator spawns replay workers at layer 2 and layer 3: new
   ASSIGNED branches `loom/moss-api-endpoints-v2` and
   `loom/ratchet-frontend-v2`, each with the same `Stack-Id` and
   `Stack-Position`, each with `Stack-Supersedes` pointing at their
   predecessor. Moss and ratchet re-run, this time against the fixed
   base, and land new COMPLETED commits.

The old branches stay in history as a record of the pre-fix state. The
new branches are the active members of the stack. The `stack/*`
projector (phase 6) picks the latest COMPLETED per position and
publishes the fixed stack to `gh-stack`.

### Phase 6 — Projection to `gh-stack`

Once all three layers are COMPLETED and integrated, bitswell invokes
the new `stack-project` recipe:

```bash
# Pseudocode; lives in a new loom-tools recipe
loom stack-project --stack-id 8f3b2e1a-4d6c-4e3b-9a1f-7c2d3e4f5a6b
```

The recipe:

1. `git log --all --grep='Stack-Id: 8f3b2e1a-4d6c-4e3b-9a1f-7c2d3e4f5a6b'
   --format='%H%n%(trailers)'` enumerates every commit tagged with
   the stack.
2. Groups by `Stack-Position`. Picks the latest COMPLETED per
   position (i.e., `loom/moss-auth-middleware-fix` for 1,
   `loom/moss-api-endpoints-v2` for 2, `loom/ratchet-frontend-v2`
   for 3).
3. Creates three projection branches:
   ```
   stack/auth-stack/01-auth-middleware  ← cherry-picks from loom/moss-auth-middleware-fix
   stack/auth-stack/02-api-endpoints    ← cherry-picks from loom/moss-api-endpoints-v2
   stack/auth-stack/03-frontend         ← cherry-picks from loom/ratchet-frontend-v2
   ```
4. `gh stack init --adopt stack/auth-stack/01-auth-middleware
   stack/auth-stack/02-api-endpoints stack/auth-stack/03-frontend` —
   per SKILL.md, the `--adopt` flag is the canonical path for
   turning existing branches into a stack.
5. `gh stack submit --auto --draft` — creates three draft PRs with
   auto-generated titles, each base-targeted at the layer below.

The three draft PRs appear on GitHub. Reviewers see exactly the layer
they are reviewing. When each PR is approved and merged via the GitHub
UI (per SKILL.md limitation 4: CLI merge is not supported), the
reviewer's merge lands on trunk. `gh stack sync` brings the rest of the
stack up to date.

Critically: **all of this activity is in the `stack/*` namespace**.
`loom/*` branches are never touched. If `gh stack rebase` has to rewrite
`stack/auth-stack/02-api-endpoints` during a sync, the corresponding
`loom/moss-api-endpoints-v2` branch is unaffected. The audit trail in
LOOM's namespace is preserved forever.

### Phase 7 — `dag-check` output

During phase 1 the orchestrator runs `dag-check` with the stack-aware
inputs to verify the plan:

```json
{
  "agents": [
    {
      "id": "ratchet/auth-middleware",
      "dependencies": [],
      "stackId": "8f3b2e1a-4d6c-4e3b-9a1f-7c2d3e4f5a6b",
      "stackPosition": 1
    },
    {
      "id": "moss/api-endpoints",
      "dependencies": ["ratchet/auth-middleware"],
      "stackId": "8f3b2e1a-4d6c-4e3b-9a1f-7c2d3e4f5a6b",
      "stackPosition": 2
    },
    {
      "id": "ratchet/frontend",
      "dependencies": ["moss/api-endpoints"],
      "stackId": "8f3b2e1a-4d6c-4e3b-9a1f-7c2d3e4f5a6b",
      "stackPosition": 3
    }
  ]
}
```

`dag-check` returns:

```json
{
  "ok": true,
  "integrationOrder": [
    "ratchet/auth-middleware",
    "moss/api-endpoints",
    "ratchet/frontend"
  ],
  "violations": []
}
```

If the orchestrator had mistakenly set `ratchet/frontend`'s position to
2 while leaving moss at 2, the new `dag-stack-order-violation` rule
would fire:

```json
{
  "ok": false,
  "integrationOrder": [...],
  "violations": [
    {
      "rule": "dag-stack-order-violation",
      "detail": "Stack-Id 8f3b2e1a-...: 'moss/api-endpoints' (position 2) integrates before 'ratchet/frontend' (position 2)"
    }
  ]
}
```

— loud, named, at plan time, before any code is written.

---

## 8. Risks and rejected alternatives

### 8.1 Risks

- **Schema bloat.** Four new trailers is ~25% growth in the
  universal-plus-state trailer vocabulary (`schemas.md` §§3.1-3.5
  currently define ~16 trailers). Mitigation: all four are conditional
  on stack-mode, and non-stack LOOM work never sees them. The rolling
  upgrade path is trivial: old validators ignore unknown trailers per
  `git-interpret-trailers(1)` semantics.

- **Validator drift.** `trailer-validate.ts`, `lifecycle-check.ts`,
  `dispatch.ts`, and `dag-check.ts` all grow new code paths. A bug in
  any one could let an invalid stack slip through. Mitigation: every
  new rule gets a Rust integration test per the project's testing
  preference, plus a fuzzer over random stack shapes. The test surface
  is ~7 new rules × ~3 edge cases each = ~21 tests; bounded and
  straightforward to specify.

- **Rebase reconciliation is not fully automatic.** The
  `stack-base-stale` detector flags the problem, but resolving it
  still requires the orchestrator to spawn a replay worker. This is
  **exactly** what LOOM already does for every other kind of rework —
  we are not inventing a new paradigm. But it does mean "mid-stack
  fix" is not a one-button operation; it is an orchestrator workflow
  that must be documented.

- **Global `stack-position-unique` check at dispatch.** Today, dispatch
  is mostly per-branch. Mitigation: the check is O(N) in the number
  of unintegrated ASSIGNED commits for one `Stack-Id`, which is
  bounded by the depth of a stack (realistically <10). No cross-epic
  global state is needed; the query is scoped to a single `Stack-Id`.

- **Interaction with `Scope-Expand`.** A worker that scope-expands
  into a file owned by a downstack layer could silently break the
  stack. Mitigation: the new `scope-expand-cross-stack` rule rejects
  `Scope-Expand` paths that overlap a downstack layer's integrated
  changes for the same `Stack-Id`. Strict-mode only; non-strict
  warns.

- **Runtime-adapter conformance.** The proposal requires mcagent
  runtimes to read `Stack-*` trailers and set `STACK_*` env vars. A
  non-conforming runtime would spawn workers with no `STACK_*` vars
  and the workers would silently fail to know their base. Mitigation:
  the new `mcagent-spec.md` MUST R.19 makes this a conformance
  requirement, and a runtime-conformance test suite (outside this
  proposal's scope but signalled in the follow-up) catches
  non-conforming runtimes at adoption time.

- **`Stack-Base-SHA` on `chore(loom)` commits.** Today the orchestrator
  never writes `Stack-Base-SHA`. The integrator needs a small new
  invocation pattern. Mitigation: it is a single trailer added to a
  commit the orchestrator already creates; no new commit type is
  needed.

### 8.2 Rejected alternatives

- **Rejected #1: encode stack position in the branch name**
  (e.g., `loom/<agent>-<slug>-s01`). Rejected because it overloads
  branch names with topology, forcing a rename on every position
  change, and it breaks the clean 1:1 mapping between
  `loom/<agent>-<slug>` and assignment slugs in `schemas.md` §2.
  Every validator that parses branch names would need a new code
  path, and the `<agent>/<slug>` → `loom/<agent>-<slug>` dependency
  resolution rule would need a third column.

- **Rejected #2: a single `Stack: <epic>/<position>/<base>` trailer**
  packing all four fields into one. Rejected because:
  - One regex has to validate four invariants simultaneously.
  - Query by position (e.g., `git log --format='%(trailers:key=Stack-Position,valueonly)'`)
    becomes a string-slicing exercise instead of a direct extraction.
  - The existing trailer vocabulary favors narrow single-purpose
    trailers (`Agent-Id`, `Session-Id`, `Task-Status`, etc.), and
    packing four fields into one breaks that pattern for no gain.

- **Rejected #3: a new `.loom/stacks/<uuid>.yaml` sidecar file** to
  store stack topology out-of-band. Rejected because:
  - It creates a second source of truth alongside git.
  - It requires new synchronization primitives (when does the sidecar
    update? how is it atomic with the commit that changes stack
    state?).
  - It defeats the "git is the database" invariant the entire LOOM
    protocol is built on.
  - A worker reading a sidecar file has to know where the sidecar
    lives; a worker reading a trailer just runs `git log`.

- **Rejected #4: piggyback on `Dependencies` alone.** `Dependencies`
  already expresses ordering, so in principle you could say "a stack
  is a chain of single-dependency assignments". Rejected because:
  - `Dependencies` expresses a DAG with possible fan-out; stacks
    assert a **linear chain** — a strictly stronger constraint.
  - `gh-stack` itself requires strict linearity (SKILL.md limitation
    1); the protocol must be able to express that constraint
    natively, not rely on a convention that the orchestrator will
    always set up single-dependency chains.
  - You still need a way to express *position* (which layer are you
    in?); `Dependencies` does not carry that.
  - The `stack-dependencies-consistency` rule keeps the two in sync:
    if `Stack-Base: <agent>/<slug>`, then `Dependencies` MUST
    include the same ref. Both are present; neither is redundant.

- **Rejected #5: let `gh-stack` be the source of truth for topology,
  and make LOOM read from its stack files.** Rejected because:
  - `gh-stack`'s stack files live in `.git/stacks/`, outside of commit
    history.
  - A LOOM worker would have to read the stack file to know its layer,
    which means the "git is the database" invariant now requires
    "plus this sidecar file".
  - Multi-worktree setups (which LOOM relies on) make stack-file
    synchronization harder — each worktree has its own
    `.git/stacks/`.
  - Every validator would need a `gh-stack`-specific code path to
    decode the stack file. Trailers are universal; stack files are
    tool-specific.

- **Rejected #6: make stack-mode the default**, and require every
  epic to be a stack. Rejected because single-assignment epics are
  common and do not benefit from stack topology. The added friction
  of mandatory stack trailers on every ASSIGNED commit is not worth
  the hypothetical uniformity.

### 8.3 Known follow-ups, out of scope for this proposal

- **`Stack-Supersedes` trailer.** Used in §7 phase 5 to let
  `stack-position-unique` ignore retired branches during mid-stack
  rework. Defined here but not implemented; a thin extension in a
  separate PR. Without it, the primary path (no rework) is
  fully functional; rework requires a manual orchestrator intervention.

- **`stack-project` recipe.** The projector from §5 is a new
  loom-tools recipe (not a new tool). Scoped out of this proposal as
  a "second PR" deliverable — the trailer schema is the foundation,
  and the projector consumes it.

- **Runtime conformance test suite for mcagent-spec R.19.** The spec
  change is in this proposal; the test suite that verifies runtimes
  actually propagate `STACK_*` env vars is a follow-up.

- **Automated stack visualization.** A new read-only tool that walks a
  `Stack-Id` group and renders a tree to stderr (similar to
  `gh stack view --json`) would be a useful ergonomic addition, but
  is not required for the primary workflow.

---

## 9. Summary

Team 3's thesis is that **stack topology is a protocol fact, not a
tooling convention**. Four new trailers (`Stack-Id`, `Stack-Position`,
`Stack-Base`, `Stack-Epic`), plus one completion trailer
(`Stack-Base-SHA`), plus seven new validation rules, plus one
conformance MUST, give LOOM first-class awareness of stacks without
breaking a single existing invariant.

The `loom/*` namespace is untouched. The `--no-ff` audit trail is
untouched. Branch names are untouched. The state machine is untouched.
The `Session-Id` model is untouched. What is added is a bounded,
conditional, auto-propagating schema extension that lets dispatchers,
validators, and the DAG planner reason about stacks directly — instead
of re-deriving them on every invocation from branch names, filesystem
conventions, or sidecar files.

`gh-stack` itself becomes a thin projection layer in a disjoint
`stack/*` namespace, fed by a read-only projector that walks
`Stack-Id` groups. Workers never run `gh stack` commands. The
orchestrator runs them once, at integration time, as the audit trail
fans out into the GitHub review workflow.

The cost is a bigger schema and a larger implementation diff. The
payoff is that every sharp edge the RFP raises — rebase vs merge,
worker authority, mid-stack rework, DAG fan-out — collapses into a
single rule in a single validator. Pay the protocol debt now; pay it
once; pay it in the place the protocol can enforce it.
