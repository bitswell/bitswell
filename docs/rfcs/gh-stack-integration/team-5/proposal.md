# Team 5 — Proposal: Stack-Mode Replaces `--no-ff` for Opted-In Epics

**RFP**: Epic #74 — gh-stack integration into LOOM
**Team angle**: replacement, not reconciliation
**Scope of this document**: how LOOM should integrate `gh stack` when an
epic opts in, what that replaces, and what it costs

---

## 1. Angle statement

For epics opted in via a `Stack-Mode: true` trailer on the root ASSIGNED
commit, the orchestrator SKIPS `--no-ff` merge integration entirely and
lands the epic through `gh stack rebase` + `gh stack submit` onto a
linear rebased history; the stack itself becomes the audit trail,
replacing the merge-commit trail for that epic.

In one sentence: **stack-mode epics have no merge commits on `main`**.
The orchestrator's `integrate-epic` path for such epics calls
`gh stack init --adopt`, `gh stack rebase`, `gh stack submit --auto`,
`gh stack sync`, and finally stamps a signed `stack-landed/<epic-slug>`
tag as the integration anchor. `pr-merge`, the workspace
`git merge --no-ff`, and the `git log --first-parent main` audit
projection are all bypassed for this epic class.

Opt-in is per-epic, declared once on the epic's root ASSIGNED commit.
It cannot be changed mid-epic, it cannot be declared per-agent, and it
cannot be retrofitted after work has started. Merge-mode epics retain
their current integration path unchanged. The two modes coexist in the
same repository without interfering because the choice is made at
decomposition and the only tools that branch on it are the integration
recipe, `pr-merge.ts` (which gains a short-circuit), and the two new
tools `stack-submit.ts` and `stack-land.ts`. Workers never know or care
which mode they are in, except to stamp one extra trailer.

Replacement, not augmentation: we explicitly do NOT keep `--no-ff`
merges as a parallel projection on top of the stack (that is team 1's
angle, and we reject it in §8). For stack-mode epics, the linear
rebased history is the canonical and *only* integration record.

---

## 2. Thesis

### 2.1 What `--no-ff` buys LOOM today

LOOM's current integration model is a straight-line sequence: verify
the worker is `COMPLETED`, verify the `Scope` trailer bounds the diff,
attempt merge, run validation, commit. `protocol.md` §3.3 specifies
the shape of this sequence generically ("attempt merge; on conflict,
abort"); the specific `--no-ff` prescription lives in the loom skill's
integration recipe (`skills/loom/SKILL.md`, steps 9–10 and the
`git merge --no-ff loom/<slug>` example on line 95). In operational
practice both are in play: `loom-tools/pr-merge.ts` calls
`gh pr merge --merge` (which produces a server-side merge commit on
the PR), and the workspace integration recipe does a local
`git merge --no-ff` of the worker branch into the workspace HEAD on
the orchestrator's dedicated integration worktree. Together these
produce the merge-commit bundle this proposal replaces. That merge
commit carries a specific bundle of guarantees:

1. **Epic grouping by topology.** `git log --first-parent main` lists
   exactly the merge commits, one per integrated worker branch, with
   no noise from individual worker commits. Any observer who wants to
   know "what epics landed this week" can trivially answer the question
   without touching commit trailers.
2. **A signed integration anchor.** The merge commit is emitted by the
   orchestrator in the integration worktree with the orchestrator's
   identity, and downstream consumers (CI, release-notes generators,
   dashboards) can key off that SHA as a stable identifier for the
   entire integration event.
3. **Pre-rebase branch form preserved as a second parent.** The
   `loom/<agent>-<slug>` branch's HEAD at the moment of integration is
   permanently attached to the merge commit via the second-parent
   pointer, so a reviewer can always walk back to the exact form the
   agent submitted without consulting reflog.
4. **Atomic revertability.** `git revert -m 1 <merge-sha>` removes
   the entire worker's contribution in one commit. This matters most
   when something broke in production and the goal is "get that epic
   off main in the next five minutes, we'll diagnose later."
5. **A stable first-parent projection for tooling.** Anything built on
   top of LOOM — release notes, changelogs, org dashboards, audit
   scripts — can consume `git log --first-parent main --format=...`
   and get a complete picture of integrations without any per-commit
   metadata awareness.

These are real guarantees. They are not hypothetical. Any proposal that
abandons them must account for each one.

### 2.2 Where those guarantees earn their keep

The `--no-ff` bundle is a good deal when an epic is long-running,
high-risk, or will outlive the attention of the people who wrote it.
A six-week platform migration with eight agents touching fourteen
directories is exactly the regime where `git revert -m 1` might get
called six months later by somebody who was not in the room when the
epic ran. The merge-commit trail is effectively free life insurance
for that class of work.

It is a bad deal when the epic is a three-agent feature landed in an
afternoon, the reviewer is sitting at their desk right now, and
`git log --first-parent` on this epic will never be run by anyone,
ever, because the entire thing will be forgotten about once the PR UI
shows green. In that regime, the merge commits are pure noise and the
ladder UX of stacked PRs is worth everything.

The question is how to give each epic the right regime. The answer is
**opt-in at the epic level**, because that is the exact granularity at
which the tradeoff actually moves. It does not move per-repo (a repo
has epics of both shapes). It does not move per-agent (an agent in a
stack-mode epic still does the same work). It moves per-epic, set
once, cannot-be-changed, declared at the top.

### 2.3 Why replacement, not parallel projection

Team 1's angle (post-integration projection) keeps `--no-ff` merges
and runs `gh stack` on top as a reviewability layer. It looks like a
free win — you get ladder UX plus the merge trail — but it is actually
the worst of both worlds for stack-mode epics:

- You keep the merge noise on `main` that the angle was trying to get
  rid of. `git log --first-parent main` still shows merge commits and
  reviewers still have to drill.
- You have two topologies for the same epic (the merge commits *and*
  the stack), which means any audit tool has to know which one is
  authoritative. There is no clean answer.
- You still pay the full cost of running `gh stack rebase` and its
  scope-check dance, so you get none of the simplicity benefit either.

The angle of this proposal is that stack-mode is a *mode*, not a
*view*. When an epic chooses it, that epic's integration record on
`main` is the linear rebased commits and the signed tag. Full stop.
The merge topology is gone and nothing in LOOM queries it for that
epic.

### 2.4 What we give up, named explicitly

We give up, for stack-mode epics only:

- The `--first-parent main` projection as a complete picture of
  integrations. Stack-mode epics do not appear as first-parents.
  Consumers that rely on this projection get a shim (§5.7).
- The pre-rebase form of each worker branch as part of the permanent
  history. Once `gh stack rebase` runs, the pre-rebase SHAs live only
  in reflog (local, expires within ~90 days) and the `loom/*` branches
  (retained 30 days per protocol §5.2). After that window, the
  pre-rebase form is gone forever. We accept this.
- Atomic `git revert -m 1 <merge-sha>`. Reverting a stack-mode epic
  means reverting each of its commits individually, in reverse order,
  using either a manual sequence or a new `stack-revert.ts` helper
  that reads the integration manifest from the signed tag. This is a
  non-trivial operational loss, and it is the hardest single cost of
  the angle.
- A single stable SHA per epic for downstream tooling. This is
  replaced by a stable *tag* per epic (`stack-landed/<epic-slug>`),
  which is nearly as good but requires one tooling migration.

The explicit tradeoff is: we lose a *topological* audit trail and gain
a *metadata-driven* one, plus ladder UX. For small-to-medium feature
epics the metadata trail is more queryable in practice (it survives
rebases, squashes, and cherry-picks, where first-parent topology does
not), and the ladder UX pays for itself the first time a reviewer
reads the epic.

### 2.5 What this proposal is not

This proposal is not a rewrite of LOOM. It is a carefully bounded
opt-in addition:

- It changes one existing loom-tool (`pr-merge.ts`) to short-circuit
  when it detects stack-mode.
- It adds two new loom-tools (`stack-submit.ts`, `stack-land.ts`) that
  are only called on the stack-mode path.
- It adds two trailers (`Stack-Mode`, `Epic-Id`) to `schemas.md`.
- It adds one line to the worker template (stamp `Epic-Id` on every
  commit in stack-mode).
- It adds one branch in the orchestrator's integration recipe.

Everything else — worker isolation, scope enforcement at the
pre-rebase checkpoint, dispatch, heartbeats, plan gates, terminal
states, `--no-ff` integration for all non-opted-in epics — is
unchanged.

---

## 3. What changes

Every component in LOOM that is affected, with exact file paths. If a
component is not listed, it is not changed. If the change is "no
change," we say so explicitly to make the scope provable.

### 3.1 `loom-tools/src/tools/pr-merge.ts` — short-circuit

**File**:
`/home/willem/bitswell/bitswell/repos/bitswell/loom-tools/src/tools/pr-merge.ts`

**Change**: add a pre-flight step that reads the epic's root ASSIGNED
commit trailers. If `Stack-Mode: true` is present, return an error
result immediately:

```ts
return err(
  'stack-mode-epic',
  'epic opted into stack-mode; use stack-submit instead of pr-merge',
  false, // not retryable — this is a routing error, not a transient failure
);
```

The orchestrator's integration recipe handles this error by routing to
the stack-mode branch instead of propagating a failure. `pr-merge`
does not try to be clever; it refuses and returns.

Rationale: making `pr-merge` refuse rather than silently call
`stack-submit` keeps the tool's responsibility narrow and makes the
stack-mode path visible in orchestrator logs. Every stack-mode
integration will contain a single `stack-mode-epic` error result from
`pr-merge` immediately before the `stack-submit` call, which is useful
for audit.

### 3.2 `loom-tools/src/tools/stack-submit.ts` — new tool

**File (new)**:
`/home/willem/bitswell/bitswell/repos/bitswell/loom-tools/src/tools/stack-submit.ts`

**Role**: `orchestrator` (matches `pr-merge.ts` pattern).

**Input schema**:

```ts
const StackSubmitInput = z.object({
  epicId: z.string().describe('Epic slug (matches Epic-Id trailer)'),
  branches: z.array(z.string()).describe(
    'Worker branches in topological order from dag-check',
  ),
  remote: z.string().optional().describe('Remote name (default: origin)'),
  draft: z.boolean().optional().describe('Create PRs as drafts (default: true)'),
});
```

**Output schema**:

```ts
const StackSubmitOutput = z.object({
  epicId: z.string(),
  layers: z.array(z.object({
    branch: z.string(),
    sha: z.string(),      // post-rebase HEAD
    prNumber: z.number(),
    prUrl: z.string(),
  })),
});
```

**Behavior** (in order):

1. `gh stack init --base main --adopt <branches...>` in topo order.
2. `gh stack rebase` — rebase the adopted branches onto current trunk.
   On exit code 3 (conflict), abort the rebase (`gh stack rebase
   --abort`), return `err('stack-rebase-conflict', ..., true)` — the
   orchestrator will dispatch a conflict-resolution worker (§6).
3. Run the post-rebase scope re-check (see `scope-check.ts` change
   below). If it fails, tear down the stack (`gh stack unstack
   --local`) and return an error.
4. `gh stack submit --auto --draft` (draft by default so the human
   reviewer can opt each layer into ready when satisfied).
5. Parse `gh stack view --json` to collect `{branch, sha, prNumber,
   prUrl}` for each layer and return them in topo order.

The tool is *only* called from the orchestrator's integration recipe.
It never runs in a worker worktree.

### 3.3 `loom-tools/src/tools/stack-land.ts` — new tool

**File (new)**:
`/home/willem/bitswell/bitswell/repos/bitswell/loom-tools/src/tools/stack-land.ts`

**Role**: `orchestrator`.

**Input schema**:

```ts
const StackLandInput = z.object({
  epicId: z.string(),
  layers: z.array(z.object({
    branch: z.string(),
    prNumber: z.number(),
  })),
  timeoutSeconds: z.number().optional().describe('Wait timeout (default: 1800)'),
});
```

**Output schema**:

```ts
const StackLandOutput = z.object({
  epicId: z.string(),
  tagName: z.string(),   // stack-landed/<epic-slug>
  tagSha: z.string(),    // the commit the tag points at
  manifest: z.string(),  // tag message (full integration manifest)
  mergedPrs: z.array(z.number()),
});
```

**Behavior** (in order):

1. Loop: run `gh stack sync` (which rebases remaining branches onto
   any squash-merged ancestors and detects merges). Then
   `gh stack view --json` and check each branch's `pr.state`. If all
   are `MERGED`, exit the loop. If any are still `OPEN` past the
   timeout, return `err('stack-land-timeout', ..., true)`.
2. Fast-forward local `main` to the remote (`git fetch origin main &&
   git merge --ff-only origin/main`). If fast-forward fails (which
   indicates divergence), abort with `err('main-diverged', ..., true)`.
3. Build the integration manifest: one line per layer with branch
   name, post-rebase SHA, PR number, orchestrator session-id, and
   ISO-8601 timestamp. The manifest is the full audit trail for the
   epic in one artifact.
4. `git tag -a -s stack-landed/<epic-slug> <top-sha> -m "<manifest>"`
   — signed annotated tag. Push with `git push origin
   stack-landed/<epic-slug>`.
5. Return the manifest and the tag SHA.

Step 4 is the integration anchor. It is the stack-mode replacement for
the `--no-ff` merge commit. It exists per-epic, it is signed by the
orchestrator, and `git show stack-landed/<epic-slug>` prints the full
manifest.

### 3.4 `loom-tools/src/tools/scope-check.ts` — post-rebase re-check

**File**:
`/home/willem/bitswell/bitswell/repos/bitswell/loom-tools/src/tools/scope-check.ts`

**Change**: gain a new entry point, `scopeCheckPostRebase(epicId,
branches)`, that for each branch:

1. Reads the worker's declared `Scope` trailer from the ASSIGNED
   commit on that branch (the ASSIGNED commit is reachable by walking
   the branch back to the pre-rebase checkpoint, which the
   orchestrator snapshot keeps for the duration of integration).
2. Computes `git diff --name-only <parent-post-rebase>...<layer-
   post-rebase>`.
3. Verifies every path in the diff matches the worker's `Scope`.
4. If any path is out of scope, returns an error with the offending
   paths.

Rationale: `gh stack rebase` replays commits, and in principle a
reordering during rebase could change the effective diff of a layer
(it cannot without a conflict, but we check anyway — defense in
depth). The post-rebase check runs once per layer, from the
orchestrator, against the rebased content. It is a new authority
boundary: workers cannot verify their own rebased form because they
never see it.

The original pre-rebase scope check remains in place; it is enforced
at the pre-integration checkpoint just as today. The post-rebase
check is *additional*, not a replacement.

### 3.5 `loom-tools/src/tools/dag-check.ts` — fan-out detector

**File**:
`/home/willem/bitswell/bitswell/repos/bitswell/loom-tools/src/tools/dag-check.ts`

**Change**: add a check that, if the epic is `Stack-Mode: true`,
refuses any DAG with fan-out. Specifically: no agent may list more
than one `Dependencies` entry, and no agent may be depended on by more
than one child. This enforces strict linearity at the assignment gate,
not at integration time, so stack-mode epics never get decomposed into
a shape `gh stack` cannot represent.

Rationale: `gh stack`'s known limitation #1 (SKILL.md line ~789) is
that stacks are strictly linear. Non-linear DAGs cannot be adopted.
Discovering this at integration time is the wrong time to discover it,
because workers would have already run. Discovering it at assignment
time means decomposition fails loudly with "this epic has fan-out;
use merge-mode or restructure the DAG."

### 3.6 `loom-tools/src/tools/commit.ts` and `trailer-validate.ts`

**Files**:
- `/home/willem/bitswell/bitswell/repos/bitswell/loom-tools/src/tools/commit.ts`
- `/home/willem/bitswell/bitswell/repos/bitswell/loom-tools/src/tools/trailer-validate.ts`

**Change**: add `Stack-Mode` and `Epic-Id` to the known trailer
vocabulary. Validation rules:

- `Stack-Mode: true|false` — allowed only on an epic's root ASSIGNED
  commit. Any other commit carrying it is rejected.
- `Epic-Id: <slug>` — required on every commit (worker and
  orchestrator) within a stack-mode epic. Optional in merge-mode
  epics, though we recommend adding it for consistency. Format:
  lowercase kebab-case, max 64 chars.

Validation is run at the same enforcement point as `Agent-Id` and
`Session-Id` — on worker commits and on orchestrator commits. A
worker in a stack-mode epic that forgets to stamp `Epic-Id` will have
its commit rejected at commit time, not at integration time.

### 3.7 `loom-tools/src/tools/pr-create.ts` — no change

**File**:
`/home/willem/bitswell/bitswell/repos/bitswell/loom-tools/src/tools/pr-create.ts`

**Change**: **no changes**. `pr-create` is simply not called on the
stack-mode path. `gh stack submit --auto` creates the PRs for
stack-mode epics, so `pr-create` is not involved. We leave it alone.

### 3.8 `loom-tools/src/tools/pr-retarget.ts` — no change

**File**:
`/home/willem/bitswell/bitswell/repos/bitswell/loom-tools/src/tools/pr-retarget.ts`

**Change**: **no changes**. `gh stack` handles base-branch retargeting
internally when layers are merged. `pr-retarget` is merge-mode only
and we leave it alone.

### 3.9 `loom` plugin — integration recipe

**File**:
`/home/willem/.claude/plugins/cache/loom-plugin/loom/0.1.0/skills/loom/`
(integration recipe, whichever file currently implements
`integrate-epic`).

**Change**: the recipe gains one branch at the top.

```
if epic.root_assigned.trailers['Stack-Mode'] == 'true':
    branches = dag-check(epic) # topo order
    scope-check(epic, branches) # pre-rebase
    stack-submit(epicId, branches)
    # wait for human review / approval
    stack-land(epicId, layers)
else:
    # existing merge-mode path — UNCHANGED
    for branch in dag-check(epic):
        pr-create(branch)
        pr-merge(branch, method='merge')
        pr-retarget(...)
    git merge --no-ff ...
```

The non-stack path is byte-identical to the current recipe. No
existing behavior moves. The branch is a top-level `if`, so merge-mode
epics never touch the stack-mode code.

### 3.10 Worker template

**File**: the LOOM worker prompt template (wherever it lives under
`skills/loom/` in the plugin cache).

**Change**: add one line to the "required trailers" section:

> If your assignment commit's epic has `Stack-Mode: true`, stamp
> `Epic-Id: <epic-slug>` on every commit. Otherwise ignore.

Workers do not branch their behavior on stack-mode beyond this. They
do not know or care that integration will be different. They never
call `gh stack`.

### 3.11 `schemas.md`

**File**:
`/home/willem/.claude/plugins/cache/loom-plugin/loom/0.1.0/skills/loom/references/schemas.md`

**Changes**:

- §3.3 assignment trailers: add `Stack-Mode` (boolean, optional,
  root-only) and `Epic-Id` (string, required in stack-mode).
- §4.1 ASSIGNED required trailers: `Stack-Mode` joins the optional
  set for non-root commits, but when present on the root it is
  propagated by the orchestrator into the children's `Epic-Id`.
- §5.7 orchestrator post-terminal commit: document the stack-mode
  alternative. For a stack-mode epic, the orchestrator's
  post-terminal artifact is the signed `stack-landed/<epic-slug>`
  tag, not a merge commit.

### 3.12 `protocol.md`

**File**:
`/home/willem/.claude/plugins/cache/loom-plugin/loom/0.1.0/skills/loom/references/protocol.md`

**Change**: §3.3 grows a sub-section §3.3.1 "Stack-mode epics" with
the conditional path. §8.2 "Audit trail" grows a paragraph describing
how to reconstitute the audit trail for stack-mode epics from
`Epic-Id` + signed tag.

### 3.13 Everything else — no change

- `assign`, `dispatch`, `read-assignment`, `status`, `status-query`,
  `trailer-validate`'s non-stack-mode rules, `commit`'s non-stack-mode
  rules, `wait`, `compile`, `test`, `push`, `review-request`,
  `lifecycle-check`, `compliance-check`, `ci-generate`, `repo-init`,
  `submodule`, `tool-request`: all unchanged.
- Worker isolation, worktree layout, heartbeat, budget, plan gate,
  dispatch machinery, scope enforcement (at the pre-rebase checkpoint):
  all unchanged.

---

## 4. Branch naming and scope

### 4.1 Worker branches keep `loom/<agent>-<slug>`

Worker branches in stack-mode epics use the same `loom/<agent>-<slug>`
convention as today. There is no `stack/*` namespace imposed on
workers. This matters because:

- Workers are stack-mode-agnostic. Their branch naming must not
  depend on the integration mode.
- Dispatch (`loom-dispatch`, `loom-spawn`) scans `loom/*` branches
  for work. Renaming would break dispatch.
- The `loom/` prefix already provides the namespace that `gh stack
  init -p` would otherwise create. Using `gh stack init --adopt`
  with full branch names lets us reuse the existing branches without
  renaming.

### 4.2 The orchestrator adopts, never renames

At integration time, the orchestrator runs:

```bash
gh stack init --base main --adopt \
  loom/ratchet-auth-mw \
  loom/moss-api \
  loom/drift-ui
```

The `--adopt` flag (SKILL.md line ~420) pulls existing branches into
a stack without creating new ones. The branch names are passed in full
(no `-p` prefix), in topological order from `dag-check.ts`. This is
important: the topo order of `gh stack init --adopt` determines the
layer order of the stack, and `gh stack rebase` will later replay
commits in that order.

There is no new `stack/<epic>/*` namespace. The `gh stack` state lives
in the repo's `.git/` metadata (wherever `gh stack` keeps its stack
file — see SKILL.md exit code 8, "stack is locked", which indicates a
lockfile). No new refs are created.

### 4.3 The stack is torn down after landing

After `stack-land.ts` completes and the signed tag is pushed, the
orchestrator runs `gh stack unstack --local` (SKILL.md lines ~737-763)
to remove the local stack tracking. Benefits:

- The `loom/*` branches remain (kept for 30 days per protocol §5.2),
  so a post-mortem can still inspect them.
- No stale stack locks hang around.
- If a subsequent retry is needed, `gh stack init --adopt` can
  re-adopt the same branches with the same or different ordering.

### 4.4 Scope trailer enforcement — two checkpoints

Scope is enforced at TWO points for stack-mode epics:

**Checkpoint A (unchanged)**: worker pre-rebase scope check. Same as
today. At worker terminal state, `scope-check.ts` runs on the
worker's branch diff against its base, and rejects any out-of-scope
file.

**Checkpoint B (new)**: orchestrator post-rebase scope re-check.
After `gh stack rebase` has rewritten each layer's commits, the
orchestrator re-runs `scope-check.ts` against the rebased diff. The
check compares `git diff --name-only <parent-post-rebase>...<layer-
post-rebase>` against the worker's original `Scope` trailer (read
from the pre-rebase ASSIGNED commit).

If Checkpoint B fails — which, per SKILL.md, it cannot without a
conflict, but we check anyway — the integration fails closed: tear
down the stack (`gh stack unstack --local`), emit a failed-
integration commit with `Error-Category: scope-violation-post-rebase`,
and leave the `loom/*` branches intact for debugging.

### 4.5 `Scope` and `Epic-Id` are orthogonal

A worker in a stack-mode epic declares its `Scope` the same way as
any other worker (matching file globs in its worktree). It
additionally stamps `Epic-Id: <slug>` on every commit. The two
trailers serve different purposes:

- `Scope` is a *spatial* constraint: which files this agent can
  touch. Enforced at scope-check, pre- and post-rebase.
- `Epic-Id` is a *provenance* label: which epic this commit belongs
  to. Enforced at commit time (trailer-validate), read at audit time.

Neither trailer replaces the other. The orchestrator's
`scope-check.ts` never reads `Epic-Id`; the audit shim
(`git log --grep='Epic-Id: auth-epic'`) never reads `Scope`.

### 4.6 No cross-stack branch reuse

A worker branch (`loom/<agent>-<slug>`) belongs to exactly one epic.
It can only be in one `gh stack` at a time (SKILL.md
`gh stack init --adopt` rejects branches already in a stack). If a
branch was partially processed in a failed stack integration, the
orchestrator must tear that stack down (`gh stack unstack --local`)
before re-adopting the branch in a retry.

---

## 5. Merge vs rebase

**This is the core section.** It must show, by construction, that the
linear rebased history carries every piece of information the
`--no-ff` merge trail carried for a stack-mode epic — otherwise the
angle fails.

### 5.1 What `--no-ff` carried, itemized

From §2.1, the merge commit carried five things:

- (a) **Epic grouping** via `git log --first-parent main`
- (b) **Signed integration anchor** (the merge commit itself)
- (c) **Pre-rebase worker branch form** as second parent
- (d) **Atomic revertability** via `git revert -m 1 <merge-sha>`
- (e) **Stable integration SHA** for downstream tooling

Each must be either replaced (preserved under a different mechanism)
or dropped (consciously given up with documented mitigation).

### 5.2 (a) Epic grouping — REPLACED by `Epic-Id` trailer

Every commit in a stack-mode epic carries `Epic-Id: <epic-slug>`,
enforced at commit time by `trailer-validate.ts`. To list the
commits in an epic:

```bash
git log --grep='Epic-Id: auth-epic' main
# or, more precisely, using git's trailer-value extraction:
git log --format='%H %(trailers:key=Epic-Id,valueonly)' main \
  | awk '$2 == "auth-epic" { print $1 }'
```

**Comparison with `--first-parent`**:

| Property | `--first-parent main` | `Epic-Id` trailer |
|---|---|---|
| Groups commits by integration event | Yes (one merge per group) | Yes (one trailer value per group) |
| Survives rebase | No | Yes |
| Survives squash | No | Yes |
| Survives cherry-pick | No | Yes (trailer copies) |
| Shows topological atom | Yes (one merge commit) | No (must aggregate) |
| Queryable without repo-wide walk | Yes (topology only) | Yes (git grep index) |

The trailer-based grouping is *strictly more queryable* once you
accept that the atom is conceptual, not topological. It survives
history rewrites in a way that topology cannot. It is also the right
trailer to carry regardless of stack-mode, which is why we recommend
(but don't require) it in merge-mode epics too.

The one property we lose is "topological atom." That is, in
merge-mode, you can look at a single SHA and know "this is the
integration commit for that epic"; in stack-mode, the epic is a
*set* of commits. The signed tag (§5.3) restores the single-SHA
handle at the integration-anchor level, but the top-of-history-
after-integration is a regular commit, not a merge commit.

### 5.3 (b) Integration anchor — REPLACED by signed tag

The orchestrator emits a signed annotated tag after `stack-land.ts`
completes:

```bash
git tag -a -s stack-landed/auth-epic <top-rebased-sha> \
  -m "$(cat <<EOF
Epic: auth-epic
Orchestrator-Session: 152203a2-4bff-45cf-8ee8-df307431d635
Integrated-At: 2026-04-14T19:42:07Z

Layers (topo order):
  ratchet/auth-mw  <post-rebase-sha>  PR#421
  moss/api         <post-rebase-sha>  PR#422
  drift/ui         <post-rebase-sha>  PR#423

Pre-rebase branches (retained until 2026-05-14):
  loom/ratchet-auth-mw  <pre-rebase-sha>
  loom/moss-api         <pre-rebase-sha>
  loom/drift-ui         <pre-rebase-sha>
EOF
)"
git push origin stack-landed/auth-epic
```

Properties of the signed tag:

- **Signed**: `-s` uses the orchestrator's signing key. The signature
  is verifiable by `git verify-tag stack-landed/auth-epic`.
- **Annotated**: carries a full message, not just a SHA reference.
- **Namespaced**: all stack-landed tags live under `refs/tags/stack-
  landed/*` for clean enumeration: `git for-each-ref refs/tags/
  stack-landed/`.
- **Stable**: once pushed, the tag SHA does not change (tags are
  immutable in practice, though technically force-pushable —
  downstream tooling MUST reject force-pushes on `stack-landed/*`).
- **Queryable**: `git show stack-landed/auth-epic` prints the full
  manifest.

This is the audit-trail replacement for the merge commit. It is
*one artifact per epic, signed by the orchestrator*, which is
exactly the guarantee the merge commit provided.

### 5.4 (c) Pre-rebase branch form — DROPPED with mitigation

This is the real loss. In merge-mode, the pre-rebase form of each
worker branch is permanently attached to main as the second parent of
the merge commit. It survives forever.

In stack-mode, `gh stack rebase` replays each layer's commits onto
the new base. The pre-rebase SHAs survive only in:

- **Reflog**: local only, 90-day default expiration.
- **The `loom/*` branches themselves**: retained for 30 days per
  protocol §5.2, then deletable.
- **The signed tag's manifest**: records the pre-rebase SHAs as text
  (see §5.3), so they are *referenceable* by SHA forever, but only
  *reachable* for as long as the branches exist.

After 30 days, the pre-rebase commits become unreachable and
eventually get garbage-collected. They are gone for good.

**Mitigation**: the signed tag's manifest records pre-rebase SHAs, so
an audit 6 months later can at least answer "what SHA did ratchet
originally submit on ratchet/auth-mw?" even if the commit is
unreachable. This is a degraded version of the guarantee (you get a
SHA string, not a reachable commit), which is *usually* sufficient
for audit but not sufficient for `git show`.

**When this loss is unacceptable**: don't opt into stack-mode. The
per-epic opt-in exists precisely so that epics where pre-rebase
provenance matters can stay in merge-mode.

### 5.5 (d) Atomic revert — DROPPED with helper mitigation

`git revert -m 1 <merge-sha>` reverts an entire merge in one commit.
Stack-mode has no equivalent: reverting a stack-mode epic means
reverting each of its commits in reverse order.

**Mitigation**: the `stack-landed/<epic-slug>` tag's manifest lists
the exact commits to revert, in order. A new helper
`loom-tools/stack-revert.ts` reads the tag manifest and produces:

```bash
git revert --no-commit <C5>
git revert --no-commit <C4>
git revert --no-commit <C3>
git revert --no-commit <C2>
git revert --no-commit <C1>
git commit -m "revert: auth-epic

Reverts stack-landed/auth-epic per manifest.

Epic-Id: auth-epic
Revert-Of: stack-landed/auth-epic
"
```

This produces a single revert commit containing the inverse diff of
every layer. It is not *atomic* in the strict git-topology sense, but
it is *operationally atomic* (one commit, one push, one rollback).

**What we lose from the atomic-revert property**:

- Ease. `git revert -m 1 <sha>` is one command, one mental model.
  `stack-revert.ts` is a helper, which means users must know the
  helper exists and which tag to point it at.
- Reviewability of the revert. Reviewing a `-m 1` revert is trivial:
  you see "revert merge commit X" and you know what happened.
  Reviewing the multi-commit revert is reading five `git revert`s
  rolled into one commit, which is harder to read but not
  impossible.

**What we gain**:

- Partial reverts. If only the top layer needs to go, `stack-revert
  --from C5 --to C4` reverts just the top. Merge-mode has no clean
  way to do this (`git revert -m 1` of a single merge reverts the
  whole merge, not just part of it).

### 5.6 (e) Stable integration SHA — REPLACED by stable tag

Downstream tooling (release-notes generators, CI dashboards, org
analytics) currently keys off the merge SHA. For stack-mode epics,
it keys off the `stack-landed/<epic-slug>` tag SHA instead.

The tag SHA is stable (the tag is immutable in practice), and
`git rev-parse stack-landed/auth-epic` returns the same SHA across
runs. This is a direct replacement with one migration step: the
downstream tool learns one new grep pattern
(`stack-landed/<slug>`) and gains the ability to query by epic slug
directly without topology walks.

### 5.7 What breaks for audit consumers, and the shim

Audit consumers that rely on specific properties of the merge trail
will observe changes for stack-mode epics:

1. **`git log --first-parent main` consumers**: stack-mode epics
   appear as a run of regular commits, not as first-parent merges.
   Any tool that lists epics by walking first-parents will *miss*
   stack-mode epics entirely (they are not first-parents of
   anything).

   **Shim**: a new `loom-tools/epic-list.ts` tool that unions
   `git log --first-parent main --format='%H %s'` with
   `git for-each-ref --format='%(objectname) %(refname:short)'
   refs/tags/stack-landed/`. Downstream tools call `epic-list` instead
   of raw `git log --first-parent`. This is the migration path.

2. **Merge-count dashboards**: any dashboard that counts merges as a
   proxy for work integrated will *undercount* stack-mode work (zero
   merges per stack-mode epic). Documented, with a recommendation to
   migrate to `Epic-Id` counting:
   `git log --format='%(trailers:key=Epic-Id,valueonly)' | sort -u
   | wc -l`.

3. **`git blame` scripts**: any script that assumed `git blame`
   points at a merge commit is broken. On stack-mode commits, blame
   points at the rebased worker commit. This is actually an
   *improvement* for debugging (the blame is now the agent's actual
   commit, not a merge), but it is a behavior change that scripts
   assuming merge-blame must be updated for.

4. **`git log --merges main`**: lists merge commits. Will show only
   merge-mode epics. Same shim as (1) applies — extend to union with
   `stack-landed/*` tags.

None of these break for merge-mode epics. The shim is opt-in at the
downstream tool level; if a tool has no stack-mode epics in its repo,
it does not need to migrate yet.

### 5.8 Why linear is preferred in stack-mode, by construction

The above accounts for what we give up. This paragraph accounts for
what we gain, which is the *reason* the replacement is worth making:

- **Review ladder visible in GitHub UI**. `gh stack submit` creates a
  real stack of linked PRs in GitHub. Reviewers see ratchet/auth-mw
  as PR#421, moss/api (based on ratchet/auth-mw) as PR#422, and
  drift/ui (based on moss/api) as PR#423. Each PR shows only its
  *layer's* diff (e.g., PR#422 shows only moss's additions, not
  ratchet's). This is the #1 complaint about merge-mode in practice
  ("I can't review the layers").

- **Linear `git log` on main**. Post-landing, `git log main` shows
  commits in dependency order with no merge-commit noise. A reviewer
  reading the epic 3 months later (or right now, for an approval
  pass) sees the same ordered sequence the workers produced.

- **`git blame` points at the agent**. Not at a merge commit. Every
  hunk of stack-mode code traces directly to the worker who wrote it.

- **No "merge-of-a-merge-of-a-merge" topology**. Long merge-mode
  epics produce deep topology on main that is hard to visualize.
  Stack-mode epics produce flat linear history.

For the target regime (small-to-medium feature epics), these
benefits are strictly worth the costs listed in §5.2–§5.7. That is
the thesis of the angle, and §6 shows it in practice.

---

## 6. Worker authority

### 6.1 Workers never invoke `gh stack`

The most important invariant in this proposal: **workers never run
`gh stack` commands**. Not in stack-mode, not in merge-mode, not in
any mode. Every `gh stack` invocation is run by the orchestrator, in
the orchestrator's dedicated integration worktree, from the
`stack-submit.ts` and `stack-land.ts` tools.

This preserves protocol.md §6.1 "Trust boundary":

> Workspace write: Only the orchestrator writes to the workspace.
> Agents MUST NOT.

`gh stack init --adopt`, `gh stack rebase`, `gh stack submit`, and
`gh stack sync` all rewrite branch state (cascade rebases, force-pushes)
and therefore count as workspace writes under LOOM's model. Only
the orchestrator performs them.

### 6.2 The only new worker obligation

In stack-mode epics, workers have *one* added responsibility: stamp
`Epic-Id: <epic-slug>` on every commit. That is it. No new tool
calls, no new commands, no awareness that integration will differ.

The worker template's one added line (§3.10) is:

> If your assignment commit's epic has `Stack-Mode: true`, stamp
> `Epic-Id: <epic-slug>` on every commit. Otherwise ignore.

The `Epic-Id` is available on the worker's ASSIGNED commit (the
orchestrator stamps it there when decomposing). The worker reads its
own ASSIGNED commit at startup (§4.2 of protocol.md — "Reads the
ASSIGNED commit from its branch for the task spec") and propagates
the trailer.

Validation (`trailer-validate.ts`) enforces this at commit time. A
worker in a stack-mode epic that forgets to stamp `Epic-Id` has its
commit rejected before it hits the branch. This is the same
enforcement mechanism as `Agent-Id` and `Session-Id`.

### 6.3 New authority boundary: post-rebase scope check

The orchestrator gains one new authority: running the post-rebase
scope re-check (§4.4, §3.4). This is a *new* authority in the sense
that no agent (worker or orchestrator) previously had "verify the
rebased form of a layer still matches the worker's declared scope"
as a responsibility.

Why only the orchestrator can do this:

- Workers cannot. They never see the rebased form (the rebase
  happens post-worker-terminal, in the orchestrator's integration
  worktree).
- No external tool can. The rebased form is only reachable in the
  orchestrator's worktree for the duration of integration.
- The check must be trustworthy (it is a scope-enforcement check),
  so it must be run by the one entity that is trusted for scope
  enforcement: the orchestrator.

### 6.4 LOOM invariants: preserved, relaxed, or broken

| Invariant | Status in stack-mode | Notes |
|---|---|---|
| Only orchestrator writes workspace | **preserved** | orchestrator runs all `gh stack` in its own worktree |
| Agents commit only to own branch | **preserved** | workers don't know about stack; commit as today |
| Scope trailer bounds worker diff | **preserved, strengthened** | enforced at pre-rebase AND post-rebase |
| Every commit has Agent-Id + Session-Id | **preserved** | unchanged |
| Workers don't talk to each other | **preserved** | unchanged |
| State-change commits have Task-Status | **preserved** | unchanged |
| Integration produces a merge commit | **broken (by design)** | stack-mode epics produce rebased commits + signed tag |
| `git log --first-parent main` = epic list | **broken (by design)** | use `epic-list.ts` shim |
| Worktree isolation | **preserved** | orchestrator worktree is separate from worker worktrees |
| Pre-rebase branches retained 30 days | **preserved** | protocol §5.2 unchanged |
| DAG must be a DAG | **tightened** | stack-mode DAG must be strictly linear (no fan-out) |
| Plan gate | **preserved** | unchanged |
| Dispatch scans `loom/*` | **preserved** | unchanged |

Two invariants are intentionally broken: the merge-commit integration
invariant and the `--first-parent` audit invariant. These are the
things we are replacing. Every other invariant is preserved or
strengthened. No invariant is silently relaxed.

### 6.5 Conflict resolution during rebase

If `gh stack rebase` hits a conflict (exit code 3 per SKILL.md), the
orchestrator does NOT attempt to resolve it. The orchestrator:

1. Aborts the rebase (`gh stack rebase --abort`).
2. Emits a terminal commit on the stuck layer with `Task-Status:
   BLOCKED`, `Blocked-Reason: rebase-conflict`, and a
   `Conflict-Files` trailer listing the conflicted paths.
3. Dispatches a new worker (a fresh session, not a resumption of the
   original) with a `Conflict-Resolution` task on the pre-rebase
   branch. That worker resolves the conflict, commits, and reaches
   terminal state.
4. The orchestrator retries the stack: tears down with `gh stack
   unstack --local`, re-runs `gh stack init --adopt`, and re-runs
   `gh stack rebase`.

**Why this is the right design**: it preserves the "only workers edit
code" invariant through the rebase machinery. The orchestrator never
touches source files — not even during a rebase conflict. All code
edits flow through a worker with a proper identity, scope, session,
and audit trail. The conflict resolution is visible in the epic's
commit history (on the worker's branch) rather than hidden inside an
orchestrator-side rebase step.

**What we explicitly reject**: "let the orchestrator resolve rebase
conflicts." That would violate workspace-write isolation by having
the orchestrator edit source files, and it would hide conflict
resolution from the epic's commit history. We consider this
non-negotiable.

### 6.6 Worker-side view: unchanged

From a worker's perspective, a stack-mode epic is identical to a
merge-mode epic, with one exception: the worker stamps `Epic-Id` on
every commit. That is all. The worker's:

- Worktree layout is the same.
- Dispatch mechanism is the same (`loom-dispatch` spawns workers
  from ASSIGNED commits on `loom/*` branches).
- Commit trailers (except `Epic-Id`) are the same.
- Scope enforcement is the same (from the worker's point of view;
  the orchestrator runs an additional post-rebase check, but the
  worker never sees that).
- Terminal states are the same.
- Heartbeat is the same.

Workers can be entirely stack-mode-unaware for all purposes except
the one trailer stamp. This is deliberate: we want the cost of
stack-mode to fall on the orchestrator's code, not on every worker
implementation.

---

## 7. End-to-end example

Two 3-agent epics, same tasks, different integration modes, shown
side by side to make the contrast concrete.

**The work**: an auth feature with three layers.
- `ratchet`: auth middleware (new file `internal/auth/mw.go`)
- `moss`: API endpoints using the middleware (edit
  `internal/api/routes.go`, new file `internal/api/handlers.go`)
- `drift`: frontend dashboard consuming the API (new files under
  `web/src/dashboard/`)

Dependencies: `moss` → `ratchet`, `drift` → `moss`. Strictly linear
DAG (so stack-mode is eligible).

### 7.1 Merge-mode epic (unchanged — for contrast)

**Decomposition** (orchestrator, on `main`):

```bash
git checkout -b loom/ratchet-auth-mw main
git commit --allow-empty -m "task(ratchet): auth middleware

Implement bearer-token verification middleware at internal/auth/mw.go.

Agent-Id: bitswell
Assigned-To: ratchet
Assignment: auth-mw
Scope: internal/auth/mw.go
Task-Status: ASSIGNED
Budget: 40000
Epic: #74-auth
"

git checkout -b loom/moss-api main
git commit --allow-empty -m "task(moss): API endpoints

Wire new middleware into /api/v1/* handlers.

Agent-Id: bitswell
Assigned-To: moss
Assignment: api
Scope: internal/api/routes.go internal/api/handlers.go
Task-Status: ASSIGNED
Dependencies: ratchet/auth-mw
Budget: 40000
Epic: #74-auth
"

git checkout -b loom/drift-ui main
git commit --allow-empty -m "task(drift): frontend dashboard

Build the dashboard that consumes /api/v1/dashboard.

Agent-Id: bitswell
Assigned-To: drift
Assignment: ui
Scope: web/src/dashboard/**
Task-Status: ASSIGNED
Dependencies: moss/api
Budget: 40000
Epic: #74-auth
"
```

**Work** (three workers, in parallel where dependencies allow):

Each worker commits implementation, tests, and a terminal
`Task-Status: COMPLETED` commit with `Files-Changed`, `Key-Finding`.
Nothing special.

**Integration** (orchestrator):

```bash
# Integrate ratchet (base of the chain)
pr-create { branch: loom/ratchet-auth-mw }
# → PR #421 targeting main
pr-merge { number: 421, method: merge }
# → merge commit M1 on main

# Integrate moss
pr-retarget { branch: loom/moss-api, newBase: main }
pr-create { branch: loom/moss-api }
# → PR #422 targeting main
pr-merge { number: 422, method: merge }
# → merge commit M2 on main

# Integrate drift
pr-retarget { branch: loom/drift-ui, newBase: main }
pr-create { branch: loom/drift-ui }
# → PR #423 targeting main
pr-merge { number: 423, method: merge }
# → merge commit M3 on main
```

**Final history on main** (`git log --first-parent main`, most recent
first):

```
M3  Merge loom/drift-ui       (merge commit, 2 parents)
M2  Merge loom/moss-api       (merge commit, 2 parents)
M1  Merge loom/ratchet-auth-mw (merge commit, 2 parents)
...previous main history...
```

Each merge has a second parent pointing at the worker's terminal
commit. `git log` without `--first-parent` interleaves worker commits
into the history.

**Audit queries**:

```bash
# List the three merges:
git log --first-parent main --format='%h %s' | grep 'Merge loom/'
# → M3 Merge loom/drift-ui
#   M2 Merge loom/moss-api
#   M1 Merge loom/ratchet-auth-mw

# Revert the entire drift layer:
git revert -m 1 <M3>
```

**Artifacts**:
- 3 PRs, all targeting main (moss's PR retargeted once, drift's twice)
- 3 merge commits on main
- Worker branches retained 30 days
- Downstream tooling keys off M1, M2, M3 SHAs

### 7.2 Stack-mode epic (new path, same work)

**Decomposition** (orchestrator, on `main`):

The only differences from §7.1: `Stack-Mode: true` on the *root*
ASSIGNED commit (ratchet's, since it has no dependencies), and
`Epic-Id: auth-epic` on every ASSIGNED commit.

```bash
git checkout -b loom/ratchet-auth-mw main
git commit --allow-empty -m "task(ratchet): auth middleware

Implement bearer-token verification middleware at internal/auth/mw.go.

Agent-Id: bitswell
Assigned-To: ratchet
Assignment: auth-mw
Scope: internal/auth/mw.go
Task-Status: ASSIGNED
Budget: 40000
Epic: #74-auth
Epic-Id: auth-epic
Stack-Mode: true
"

git checkout -b loom/moss-api main
git commit --allow-empty -m "task(moss): API endpoints

Wire new middleware into /api/v1/* handlers.

Agent-Id: bitswell
Assigned-To: moss
Assignment: api
Scope: internal/api/routes.go internal/api/handlers.go
Task-Status: ASSIGNED
Dependencies: ratchet/auth-mw
Budget: 40000
Epic: #74-auth
Epic-Id: auth-epic
"

git checkout -b loom/drift-ui main
git commit --allow-empty -m "task(drift): frontend dashboard

Build the dashboard that consumes /api/v1/dashboard.

Agent-Id: bitswell
Assigned-To: drift
Assignment: ui
Scope: web/src/dashboard/**
Task-Status: ASSIGNED
Dependencies: moss/api
Budget: 40000
Epic: #74-auth
Epic-Id: auth-epic
"
```

Note: `Stack-Mode: true` appears only on ratchet's (the root). moss
and drift inherit stack-mode via the orchestrator's decomposition
logic (which knows the epic's root trailer) — they do NOT stamp
`Stack-Mode` themselves, because `trailer-validate.ts` rejects
`Stack-Mode` on non-root commits (§3.6).

All three workers stamp `Epic-Id: auth-epic` on every commit they
make (not just ASSIGNED — every commit).

**Work** (three workers, identical to §7.1 except each worker commit
carries `Epic-Id: auth-epic`):

```
ratchet's branch (loom/ratchet-auth-mw):
  C1a  "ratchet: add auth middleware skeleton"         Epic-Id: auth-epic
  C1b  "ratchet: implement token verification"         Epic-Id: auth-epic
  C1c  "ratchet: tests + COMPLETED"                    Epic-Id: auth-epic, Task-Status: COMPLETED

moss's branch (loom/moss-api):
  C2a  "moss: wire middleware into /api/v1"            Epic-Id: auth-epic
  C2b  "moss: add handlers for dashboard endpoint"     Epic-Id: auth-epic
  C2c  "moss: tests + COMPLETED"                       Epic-Id: auth-epic, Task-Status: COMPLETED

drift's branch (loom/drift-ui):
  C3a  "drift: scaffold dashboard component"           Epic-Id: auth-epic
  C3b  "drift: wire API calls + tests + COMPLETED"     Epic-Id: auth-epic, Task-Status: COMPLETED
```

**Integration** (orchestrator — new path):

```bash
# 1. Orchestrator's integration recipe detects Stack-Mode: true on root.
#    Reads epic.stack_mode = true from AGENT.json mirror or from the commit.

# 2. Topo-order the branches via dag-check.
#    Result: [loom/ratchet-auth-mw, loom/moss-api, loom/drift-ui]

# 3. Pre-rebase scope check (unchanged — per worker, against their base).
#    Passes.

# 4. Call stack-submit.ts:
gh stack init --base main --adopt \
  loom/ratchet-auth-mw \
  loom/moss-api \
  loom/drift-ui
# → stack created; drift-ui is checked out (top)

gh stack rebase
# → cascades: ratchet onto main, moss onto rebased ratchet,
#   drift onto rebased moss. No conflicts. Exit 0.

# 5. Post-rebase scope re-check (new).
#    For each layer, diff against its (rebased) parent, verify paths
#    match the worker's Scope trailer.
#    ratchet: internal/auth/mw.go ✓
#    moss: internal/api/routes.go, internal/api/handlers.go ✓
#    drift: web/src/dashboard/*.{tsx,ts,css} ✓
#    Passes.

gh stack submit --auto --draft
# → Created PR #421 for loom/ratchet-auth-mw (base: main)
#   Created PR #422 for loom/moss-api (base: loom/ratchet-auth-mw)
#   Created PR #423 for loom/drift-ui (base: loom/moss-api)
#   Linked as a Stack on GitHub.

# 6. stack-submit returns:
#    layers: [
#      { branch: "loom/ratchet-auth-mw", sha: "<R1>", prNumber: 421, prUrl: ... },
#      { branch: "loom/moss-api",        sha: "<R2>", prNumber: 422, prUrl: ... },
#      { branch: "loom/drift-ui",        sha: "<R3>", prNumber: 423, prUrl: ... },
#    ]

# 7. Human reviewer opens each PR in the GitHub stack UI, reviews each
#    layer's diff (not the cumulative diff), marks each ready, approves.

# 8. PRs get merged one at a time (from bottom up) by the human or by
#    GitHub auto-merge. Each merge:
#    - Merges the bottom PR into main.
#    - Triggers gh stack sync on the next integration-worktree poll.

# 9. stack-land.ts wait loop:
while true; do
  gh stack sync
  # → gh stack detects squash-merge of ratchet, rebases moss+drift
  #   onto new main, reruns.
  json=$(gh stack view --json)
  if all branches isMerged:true in $json; then break; fi
  sleep 30
done

# 10. Fast-forward local main:
git fetch origin main
git merge --ff-only origin/main
# → main advances to the top-of-stack SHA (C3-rebased = C5 in the
#   final-history listing below)

# 11. Build the integration manifest and tag:
git tag -a -s stack-landed/auth-epic HEAD -m "$(cat <<EOF
Epic: auth-epic
Orchestrator-Session: 152203a2-4bff-45cf-8ee8-df307431d635
Integrated-At: 2026-04-14T19:42:07Z

Layers (topo order):
  loom/ratchet-auth-mw  <R1>  PR#421
  loom/moss-api         <R2>  PR#422
  loom/drift-ui         <R3>  PR#423

Pre-rebase branches (retained until 2026-05-14):
  loom/ratchet-auth-mw  <pre-C1c>
  loom/moss-api         <pre-C2c>
  loom/drift-ui         <pre-C3b>

Commits on main (in order, bottom → top):
  <C1> ratchet: add auth middleware skeleton
  <C2> ratchet: implement token verification
  <C3> ratchet: tests
  <C4> moss: wire middleware into /api/v1
  <C5> moss: add handlers for dashboard endpoint
  <C6> moss: tests
  <C7> drift: scaffold dashboard component
  <C8> drift: wire API calls + tests
EOF
)"

git push origin stack-landed/auth-epic

# 12. Tear down local stack state (but keep loom/* branches):
gh stack unstack --local
```

**Final history on main** (`git log main`, most recent first, no
`--first-parent`):

```
C8  drift: wire API calls + tests      Epic-Id: auth-epic, Agent-Id: drift
C7  drift: scaffold dashboard component Epic-Id: auth-epic, Agent-Id: drift
C6  moss: tests                         Epic-Id: auth-epic, Agent-Id: moss
C5  moss: add handlers for dashboard    Epic-Id: auth-epic, Agent-Id: moss
C4  moss: wire middleware into /api/v1  Epic-Id: auth-epic, Agent-Id: moss
C3  ratchet: tests                      Epic-Id: auth-epic, Agent-Id: ratchet
C2  ratchet: implement token verif.     Epic-Id: auth-epic, Agent-Id: ratchet
C1  ratchet: add auth middleware skel.  Epic-Id: auth-epic, Agent-Id: ratchet
...previous main history...
```

Plus one tag:

```
stack-landed/auth-epic → C8 (signed, with manifest)
```

**Audit queries**:

```bash
# List commits in the epic:
git log --grep='Epic-Id: auth-epic' main --format='%h %s'
# → C8 drift: wire API calls + tests
#   C7 drift: scaffold dashboard component
#   ...
#   C1 ratchet: add auth middleware skeleton

# Show the integration manifest:
git show stack-landed/auth-epic
# → Prints the tag message (full manifest).

# Verify the orchestrator's signature:
git verify-tag stack-landed/auth-epic
# → Good signature by <orchestrator key>.

# Revert the entire epic (via helper):
stack-revert --tag stack-landed/auth-epic
# → Produces one revert commit that undoes C1..C8.

# Revert just the drift layer:
stack-revert --tag stack-landed/auth-epic --from C8 --to C7
# → Produces one revert commit that undoes only C7+C8.
```

**Artifacts**:
- 3 PRs, stacked (linked in GitHub's stack UI)
- 0 merge commits on main
- 8 rebased worker commits on main, in dependency order
- 1 signed tag (`stack-landed/auth-epic`) pointing at C8
- Worker branches retained 30 days
- Downstream tooling keys off the tag (or `Epic-Id` trailer)

### 7.3 The contrast, line by line

| Property | Merge-mode (§7.1) | Stack-mode (§7.2) |
|---|---|---|
| Commits on main from this epic | 3 merges (M1–M3) as first-parents; 8 worker commits as second-parents | 8 worker commits, linear, no merges |
| PR shape | 3 PRs, all targeting main | 3 PRs, stacked (each based on previous) |
| Review UI | GitHub shows 3 independent PRs | GitHub shows a 3-layer stack |
| Per-PR diff | Each PR shows its full branch diff (includes base history) | Each PR shows only its layer's diff |
| Provenance query | `git log --first-parent main` | `git log --grep='Epic-Id: auth-epic'` |
| Integration anchor | 3 merge commits | 1 signed tag |
| Single SHA for epic | No (three merge SHAs) | Yes (the tag's target SHA) |
| Atomic revert | `git revert -m 1 M3` | `stack-revert --tag stack-landed/auth-epic` |
| Partial revert | Hard (topology-bound) | Easy (per-commit via helper) |
| Pre-rebase branch form | Permanent (second parent of merge) | Retained 30 days, then recorded only in tag manifest |
| `git blame` target | Merge commits mixed with worker commits | Agent commits directly |
| Reviewer mental model | "3 big PRs to review independently" | "a ladder — read from bottom up" |

Both epics do the same work by the same workers in the same order.
The only difference is the shape of the integration record, and in
§7.2 the reviewer sees the dependency structure as a ladder in the
GitHub UI, which is the core benefit of stack-mode.

### 7.4 What a failed conflict looks like

Suppose `gh stack rebase` in step 4 of §7.2 hit a conflict in
drift's commit against moss's rebased form:

```bash
gh stack rebase
# ✗ Rebase conflict in loom/drift-ui:
#   web/src/dashboard/api.ts
# Exit code 3.

# Orchestrator:
gh stack rebase --abort
# → all branches restored to pre-rebase state
gh stack unstack --local
# → local stack removed; loom/* branches intact

# Orchestrator emits a BLOCKED commit on loom/drift-ui:
git checkout loom/drift-ui
git commit --allow-empty -m "blocked(drift): rebase conflict on dashboard/api.ts

Agent-Id: bitswell
Session-Id: <new-integration-session>
Epic-Id: auth-epic
Task-Status: BLOCKED
Blocked-Reason: rebase-conflict
Conflict-Files: web/src/dashboard/api.ts
"

# Orchestrator dispatches a new worker (drift or another writer):
loom-dispatch --branch loom/drift-ui
# → new session on drift, task: resolve the conflict
# → drift commits resolution, reaches COMPLETED

# Orchestrator retries integration from step 4 of §7.2:
gh stack init --base main --adopt \
  loom/ratchet-auth-mw loom/moss-api loom/drift-ui
gh stack rebase
# → succeeds this time
# ... proceeds to submit, land, tag
```

The conflict resolution is visible in `loom/drift-ui`'s commit
history as a regular worker commit with `Epic-Id: auth-epic` and is
therefore included in the post-rebase `git log` on main.

---

## 8. Risks and rejected alternatives

### 8.1 Risks

**R1 — An audit consumer we don't know about keys off `--first-parent main`.**

*Likelihood*: medium. LOOM has been shipping long enough that
downstream tools (release-notes generators, dashboards, compliance
reports) may key off first-parent without our knowledge.

*Mitigation*:
- Opt-in is epic-level. No existing epic is retroactively converted.
  A repo can run stack-mode epics alongside merge-mode epics
  indefinitely, and no merge-mode epic's first-parent projection
  changes.
- `epic-list.ts` shim (§5.7) provides a migration path: downstream
  tools replace `git log --first-parent main` with `epic-list`, which
  unions first-parent with `stack-landed/*` tags.
- Document the opt-in loudly at the RFP decision time. Anyone who
  adopts stack-mode for their repo does so knowing they must migrate
  their audit scripts.

**R2 — `gh stack rebase` silently drops or re-orders changes.**

*Likelihood*: low. SKILL.md states rebase detects conflicts and
restores pre-rebase state on conflict (exit code 3). But
defense-in-depth: we check.

*Mitigation*: post-rebase scope check (§3.4, §4.4) computes the
rebased diff per-layer and verifies every path matches the worker's
`Scope`. Additionally, a content hash comparison: the combined
rebased diff should match the topo-merged pre-rebase diff
byte-for-byte (modulo formatting). If not, fail closed: tear down
the stack, emit a failed-integration commit, leave `loom/*`
branches intact.

**R3 — Stack-mode is chosen for an epic that later needs atomic revert.**

*Likelihood*: low. Stack-mode is opt-in at decomposition time; the
orchestrator (or the human who kicked off the epic) makes the call
knowing the revert-cost tradeoff.

*Mitigation*:
- Document the revert-cost at opt-in: when `Stack-Mode: true` is
  set on a root ASSIGNED commit, the orchestrator prints a warning
  summarizing the tradeoff.
- `stack-revert.ts` helper reads the tag manifest and produces the
  revert sequence. Worst case (helper missing): a human reverts
  commits one by one from the manifest.
- For epics where operational revert-cost is a known concern (e.g.,
  anything touching auth, payments, data migrations): do not opt
  into stack-mode.

**R4 — `gh stack`'s strict linearity clashes with non-linear DAGs.**

*Likelihood*: certain (it is a hard constraint — SKILL.md line ~789).

*Mitigation*: `dag-check.ts` refuses to enable stack-mode on any
epic whose `Dependencies` graph has fan-out. The check runs at the
assignment gate (decomposition time), not at integration time, so
an epic that cannot be linearized fails before any worker runs.
Error message: "epic <slug> has fan-out in its dependency graph
(<list>); use merge-mode or restructure the DAG."

**R5 — Conflict resolution during rebase violates worker authority.**

*Likelihood*: would be certain if we let the orchestrator resolve
conflicts. Must not.

*Mitigation*: the orchestrator NEVER edits code during `gh stack
rebase`. Conflicts produce a BLOCKED commit and a new
`Conflict-Resolution` worker assignment (§6.5, §7.4). This
preserves the "only workers edit code" invariant through rebase.

**R6 — A stack-mode epic interleaves on main with non-stack commits.**

*Likelihood*: medium. If other work lands on main while a stack-mode
epic is in-progress, `gh stack sync` will cascade-rebase the stack
onto the new main, which can produce conflicts and delay the epic.

*Mitigation*:
- Same as for any long-running PR: small epics are less exposed to
  this, which is another reason stack-mode is recommended for
  small-to-medium epics only.
- `gh stack sync` handles the cascade automatically in the no-
  conflict case. Conflicts fall into R5's path.

**R7 — A stale `stack-landed/*` tag is force-pushed.**

*Likelihood*: low, but tags are technically force-pushable.

*Mitigation*: add a repo-level branch protection rule that rejects
force-pushes on `refs/tags/stack-landed/*`. This is a GitHub repo
setting, not a LOOM change, but we document it in the adoption
checklist.

**R8 — The orchestrator's signing key changes mid-epic.**

*Likelihood*: rare, but possible across long-running orchestrator
sessions or key rotations.

*Mitigation*: the tag's signature is point-in-time. A rotation after
the tag is pushed does not invalidate it (GPG signatures are
verifiable as long as the old key is in the keyring). Document the
key-rotation procedure as part of orchestrator operations.

### 8.2 Rejected alternatives

**Rejected Alternative 1: "Convert merge-mode epics to stack-mode at integration time."**

*The idea*: let workers run merge-mode (no `Epic-Id` stamping), and
at integration time, the orchestrator decides whether to do a merge-
mode integration or a stack-mode integration. Free ladder UX for
all epics.

*Why we reject*:

- Requires the orchestrator to stamp `Epic-Id` on worker commits
  *after* the fact. Protocol §2 allows orchestrator post-terminal
  commits, but retrofitting trailers on past worker commits means
  rewriting those commits, which changes their SHAs. The
  `Task-Status: COMPLETED` commit's SHA is a stable reference in
  audit queries; rewriting it is worse than not having the trailer.
- Alternatively, the orchestrator could skip `Epic-Id` and rely on
  first-parent grouping or the signed tag alone. But then the
  `Epic-Id`-based audit shim (§5.7) doesn't work — because only some
  epics have `Epic-Id` trailers, and the others are grouped only by
  topology. The audit story fragments.
- Decision-time ambiguity. If an epic can be integrated in either
  mode, a reviewer reading it mid-work doesn't know whether to
  expect a merge commit or a rebased sequence. The review UX
  depends on the decision.

Cleaner to decide at decomposition, when the orchestrator has full
knowledge of the DAG and the consumer expectations. Reject.

**Rejected Alternative 2: "Keep `--no-ff` merges AND run `gh stack` as a parallel projection."**

*The idea*: do a normal merge-mode integration (merges on main),
AND also run `gh stack submit` so reviewers get the ladder UX in
GitHub. Best of both worlds.

*Why we reject*:

- The whole point of stack-mode is that the rebased history is the
  canonical integration record. A parallel projection gives you
  neither clean history (merges remain) nor a single source of
  truth (two topologies exist, and any audit tool must know which
  is authoritative).
- Running two integration paths doubles the surface area for
  bugs. A failure on the stack projection after a successful merge
  integration leaves the repo in a confusing half-state.
- This is the right idea for team 1's angle (review-layer projection,
  where the stack is a view, not a record). For team 5's angle
  (replacement), it is hedging. The RFP asks each team to commit to
  an angle, and we commit to replacement.

**Rejected Alternative 3: "Stack-mode at the agent level, not the epic level."**

*The idea*: individual agents within an epic opt into stack-mode.
Fine-grained control, maximally flexible.

*Why we reject*:

- A mid-chain agent in stack-mode depending on a merge-mode parent
  needs its pre-rebase SHA to be an ancestor of main. In merge
  mode, the parent merged as `--no-ff`, so the worker branch's
  pre-rebase form is a second parent of the merge, not an ancestor
  of main. `gh stack init --adopt` would refuse the mid-chain
  worker because its parent branch isn't on `main`'s history in
  the required way.
- Even if we resolve the topology issue, the review UX is
  incoherent: some layers are merge-mode PRs, some are stack-mode
  PRs, and the GitHub stack UI can't represent the mix cleanly.
- Epic-level opt-in matches how the provenance tradeoff actually
  moves. An epic is coherent: either it wants the rebased ladder
  UX or it wants the merge trail, not both.

**Rejected Alternative 4: "Make stack-mode the default and merge-mode the opt-out."**

*The idea*: since small-to-medium feature epics are the common case,
flip the default.

*Why we reject*:

- Rollout risk. Every existing audit script, every downstream tool
  that keys off merge topology, every dashboard: all break on the
  first post-change epic.
- LOOM's audit story is one of its core value propositions for
  long-running, high-stakes work. Defaulting to the lossier model
  sells that value out by default.
- Opt-in is conservative, reversible, and per-epic. A repo that
  wants stack-mode as the default can set a repo-level policy
  ("prefer stack-mode for epics with <10 agents and no fan-out")
  in their own orchestrator logic. But LOOM the protocol does not
  impose that choice.

**Rejected Alternative 5: "Represent the stack as a single squash-merge into main."**

*The idea*: instead of rebasing commits onto main, squash the entire
stack into one commit and merge that. One commit per epic, very
clean history.

*Why we reject*:

- Loses `git blame` per-agent. A squash collapses all authors.
- Loses per-worker commit metadata (`Agent-Id`, `Session-Id`) on
  individual changes. Only the top-level squash has trailers.
- Defeats the entire ladder-review benefit: if the epic is landing
  as one commit, there is no ladder for the reviewer.
- Makes partial revert impossible.

Squash is a fine default for a one-agent feature PR. It is wrong
for a multi-agent epic where per-agent provenance matters.

### 8.3 What this proposal is betting on

The core bet: **for small-to-medium feature epics, ladder UX +
metadata-driven audit is strictly better than merge topology + no
ladder, and the per-epic opt-in lets us take this bet only where it
pays off**.

If that bet is wrong — if reviewers don't actually find the ladder
UX more valuable than current PR review, or if the audit shim turns
out to be a pain in practice, or if pre-rebase provenance matters
more than we think — then the fallback is clean: don't opt epics
into stack-mode. Merge-mode is unchanged and continues to work
exactly as today.

If the bet is right, stack-mode becomes the preferred mode for the
common case, and long-running high-stakes epics keep merge-mode as
a deliberate choice. Both modes coexist without interfering.

---

## 9. Summary

Stack-mode is an opt-in integration mode that replaces `--no-ff` for
epics that declare `Stack-Mode: true` on their root ASSIGNED commit.
For those epics:

- Workers work exactly as today, with one added trailer (`Epic-Id`).
- The orchestrator adopts worker branches into a `gh stack`, rebases,
  submits, waits for human review, syncs on merge, and stamps a
  signed `stack-landed/<epic-slug>` tag as the integration anchor.
- No merge commits land on main for the epic.
- Audit goes through `Epic-Id` trailers and the signed tag, not
  first-parent topology.
- Revert is per-commit via a helper; atomic merge-revert is lost.

Merge-mode epics are unchanged. The two modes coexist per-epic. The
opt-in cannot mix within one epic. The proposal adds two new tools
(`stack-submit`, `stack-land`), one short-circuit in `pr-merge`, one
post-rebase check in `scope-check`, one fan-out gate in `dag-check`,
two new trailers, one worker-template line, and a branch in the
orchestrator's integration recipe. Every other LOOM component is
unchanged.

The bet is that small-to-medium feature epics will find the ladder
UX and rebased history strictly more valuable than the merge-commit
audit trail, and the per-epic opt-in lets us take that bet only
where it pays off.

---

*End of team 5 proposal for RFP #74.*
