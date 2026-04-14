# Team 1 — Proposal: Stacks as a Read-Only Presentation Layer

## 1. Angle statement

**Stacks are a read-only presentation layer: LOOM keeps merging as today, and the orchestrator projects a shadow `stack/*` mirror namespace via `gh-stack` purely for reviewer UX.**

This is the deliberate opposite of using `gh-stack` as the *integration mechanism*. In our model `gh-stack` is an **output artifact** — a second, disposable view of the same DAG, rebuilt on demand from the existing `loom/<agent>-<slug>` branches. The canonical history still lives in the workspace, still arrives there via `--no-ff` merges, and still passes through the existing per-agent `Scope:` checks. The stack PRs are a ladder-shaped projection of that history, not a parallel truth.

The invariant this angle protects, above all, is LOOM's `--no-ff` merge audit trail. The explicit cost we pay: stack PRs are disposable. Reviewers approve them in the sense of "I am happy with this code to flow into the canonical `loom/*` branch"; the orchestrator does not *merge* the stack PR as an integration event. Approval is a signal; integration is still LOOM's job.

Put differently: in this proposal `gh-stack` is not plumbing. It is a lens.

---

## 2. Thesis

### 2.1 The sharp edges are shaped by one assumption

Re-read the five sharp edges in issue #74. Three of them — merge vs rebase (#4), worker authority (#5), and branch-namespace ownership (#3) — are only sharp *if we try to make `gh-stack` the integration mechanism*. They all dissolve the moment `gh-stack` is demoted to an output layer.

- **Merge vs rebase** is a conflict only if the rebased branches *are* the audit trail. If the rebased branches are a throwaway mirror, rebasing them is fine; force-pushing them is fine; tearing them down and regenerating them is fine. The audit trail lives somewhere else.
- **Worker authority** is only a hard question if workers need to run `gh stack submit` to make their branch visible. If the orchestrator projects the whole stack in one go, after all workers are COMPLETED and integrated, workers never touch `gh stack` at all.
- **Branch namespace ownership** is only contested if two systems must co-own `loom/<agent>-<slug>`. If `gh-stack` owns a separate `stack/*` namespace and `loom/*` stays untouched, there is nothing to arbitrate.

That leaves two sharp edges for this proposal to actually answer: the fact that `loom-tools` already accepts custom bases (#1) and the fact that the DAG already encodes order (#2). Both of those are *reusable primitives*, not problems. This angle is built on top of both of them.

### 2.2 Why this is the right bet

The bet is that stacks are worth their UX value but *not* worth paying for with LOOM's debuggability. LOOM's merge-based workspace is the thing that makes orchestrator post-mortems tractable. When a reviewer comes back a week later and asks "why did agent X land with this exact blob?", the answer today is one `git log --first-parent` away on the workspace. That property is load-bearing. Any proposal that breaks it (by replacing `--no-ff` with rebase, or by letting workers force-push their own PRs) trades a durable invariant for a UX improvement. That trade is worse than it looks: LOOM runs many agents, blast radius matters, and "force-push broke a trailer" is the kind of bug that destroys a whole afternoon.

The inverse trade — spending a narrow orchestrator recipe and one new tool to get the ladder-of-diffs UX *without* touching the invariants — is cheap. The `stack-project` tool is under 100 lines. The orchestrator recipe is a handful of steps inside the existing post-integration phase. The protocol surface does not grow. The worker template does not grow. The schemas do not grow. No new trailers. No new roles. No new lifecycle states.

We get the UX for the cost of a view.

### 2.3 What the angle gives up

Honesty first: this proposal does *not* give reviewers a merge button on the stack. When a reviewer approves `stack/epic-slug/layer-2`, nothing lands. The stack PRs are drafts; the orchestrator closes them on final approval and runs the normal LOOM integration path. That is a real UX cost: reviewers must understand that "approval on the mirror flows through to the source." We mitigate it with a `[MIRROR]` PR-title prefix and a standardized PR body template — but it is still a cognitive tax.

It also does not give us *incremental* stacking mid-epic. Workers still land into the workspace in DAG order as whole commits per agent. You cannot have two half-done layers reviewed in parallel on the stack. If a team wants that, they want a different proposal.

What it does give us is a cheap, invariant-preserving, reversible path to the stacked-review UX, with an escape hatch if `gh-stack` turns out to be the wrong bet (delete the recipe, delete the tool, zero cleanup in the workspace).

---

## 3. What changes

This section enumerates every component in the LOOM surface area and states whether it changes. When nothing changes, we say so explicitly — that is the point of the angle.

### 3.1 `loom-tools` — one new tool

**New file: `repos/bitswell/loom-tools/src/tools/stack-project.ts`.**

Shape, mirroring `pr-create.ts` and `pr-retarget.ts`:

```ts
import { z } from 'zod';
import type { Tool } from '../types/tool.js';
import { ok, err } from '../types/result.js';
import { exec } from '../util/exec.js';

const StackProjectInput = z.object({
  epic: z.string().describe('Epic slug; used for stack/<epic>/... namespace'),
  order: z.array(z.string()).describe(
    'Integration order of loom/* branches, DAG-sorted. First entry lands ' +
    'at layer 01, closest to trunk.'
  ),
  base: z.string().default('main').describe('Trunk branch for the stack'),
  draft: z.boolean().default(true).describe('Publish stack PRs as drafts'),
});

const StackProjectOutput = z.object({
  mirrorBranches: z.array(z.string()),
  prUrls: z.array(z.string()),
});
```

`roles: ['orchestrator']`, matching the precedent set by `pr-create.ts` and `pr-retarget.ts`. The handler:

1. Accepts an already-DAG-sorted `order` (callers are expected to pipe `dag-check`'s `integrationOrder` into it).
2. For each entry, computes mirror branch name `stack/<epic>/<NN>-<agent>-<slug>` with `NN` as zero-padded layer index.
3. Rebuilds mirror branches from the workspace: for each layer, the tool fast-forwards a fresh branch to the merge commit that integrated that layer in the workspace (found via `git log --first-parent --grep="Assigned-To: <agent>"`).
4. Runs `gh stack init --base <base> --adopt <mirror-1> <mirror-2> ...` to adopt the mirror set into a stack.
5. Runs `gh stack submit --auto --draft` (drops `--draft` only if `input.draft` is false) to push and create stack PRs.
6. Returns the list of created mirror branches and PR URLs.

The tool is deliberately thin — it is a recipe expressed as TypeScript, not a new protocol. If a step fails, it bails without leaving a half-published stack on GitHub (covered in §3.5).

### 3.2 `loom-tools` — no changes to existing tools

Explicitly:

- `repos/bitswell/loom-tools/src/tools/pr-create.ts` — **no changes**. Keeps operating on `loom/*` branches with the base argument it already accepts.
- `repos/bitswell/loom-tools/src/tools/pr-retarget.ts` — **no changes**. Still the escape hatch if someone wants to retarget a `loom/*` PR's base manually.
- `repos/bitswell/loom-tools/src/tools/commit.ts` — **no changes**.
- `repos/bitswell/loom-tools/src/tools/push.ts` — **no changes**.
- `repos/bitswell/loom-tools/src/tools/dag-check.ts` — **no changes**. Its existing output field `integrationOrder` is exactly what `stack-project` consumes.
- `repos/bitswell/loom-tools/src/tools/trailer-validate.ts` (and any other trailer validator) — **no changes**. No new trailers to validate.

This "no changes" list is the value of the angle: *the load-bearing parts of `loom-tools` are untouched*. All risk is concentrated in one new file.

### 3.3 `loom` plugin skill — one new recipe

**Modified file:** `~/.claude/plugins/cache/loom-plugin/loom/0.1.0/skills/loom/references/protocol.md` gets a new §7.2 "Stack projection recipe" describing:

- When to run: *after* all agents in an epic are COMPLETED and integrated into the workspace, *before* the orchestrator announces the epic as review-ready.
- Who runs it: orchestrator only, from the workspace (never from a worktree).
- What it runs: `stack-project` with `integrationOrder` from `dag-check`.
- Failure handling: if `stack-project` fails partway, the orchestrator tears down the mirror namespace with `gh stack unstack` and either retries or falls back to per-`loom/*` PRs via existing `pr-create`.
- Teardown on epic completion: after final merge, orchestrator deletes all `stack/<epic-slug>/*` branches and closes any lingering mirror PRs.

The recipe is documentation + a small number of tool calls. No new code in the plugin.

### 3.4 Worker template — no changes

`~/.claude/plugins/cache/loom-plugin/loom/0.1.0/skills/loom/references/worker-template.md` — **no changes**.

Workers never see `gh stack`, `gh-stack`, the `stack/*` namespace, or the `stack-project` tool. Their task spec, trailer vocabulary, and scope enforcement are identical to today. This is §5's argument expressed as a diff: the smallest diff that exists.

### 3.5 Schemas — no changes

`references/schemas.md` — **no changes**.

- No new trailers. Stack layer ordering is recovered from `Dependencies:` via `dag-check`, not from a new `Stack-Parent:` or `Stack-Order:` trailer.
- No new `Task-Status` values. Mirror branches are not agent branches; they have no lifecycle.
- No new branch naming rule at the schema level. `loom/<agent>-<slug>` keeps its exclusive grip on the agent namespace. `stack/<epic-slug>/<layer-n>` is a *convention* described in `protocol.md`, not a validated pattern in `schemas.md`, because nothing in LOOM's validators reads mirror branches.

This last distinction matters: the mirror namespace is deliberately kept *out* of the schema layer so that the validator surface does not grow. If a mirror branch has the "wrong" shape, nothing fails — at worst a reviewer sees an ugly PR title.

### 3.6 `loom-dispatch` — one-line exclusion

`loom-dispatch --scan` iterates `loom/*` branches. Nothing we do breaks that. We add one safety line to its branch filter to *explicitly* skip `stack/*` branches, so that an accidental `Task-Status: ASSIGNED` on a mirror branch (e.g., from a mis-typed commit) cannot spawn a worker.

This is the only change to `loom-dispatch`, it is a one-line guard, and it is defensive rather than load-bearing.

### 3.7 Trailer vocabulary — no changes

The RFP asks explicitly about this. Our answer: zero new trailers. The `Dependencies:` trailer plus `dag-check` already encode everything a stack projector needs. Adding `Stack-Parent:` or `Stack-Order:` would be redundant (the order is derivable) and would grow the protocol surface without removing anything.

### 3.8 Summary of changed files

| File | Change |
|---|---|
| `repos/bitswell/loom-tools/src/tools/stack-project.ts` | NEW — projection tool, orchestrator-role |
| `repos/bitswell/loom-tools/src/index.ts` (tool registry) | add one import + register entry |
| `repos/bitswell/loom-tools/src/tools/pr-create.ts` | no changes |
| `repos/bitswell/loom-tools/src/tools/pr-retarget.ts` | no changes |
| `repos/bitswell/loom-tools/src/tools/push.ts` | no changes |
| `repos/bitswell/loom-tools/src/tools/commit.ts` | no changes |
| `repos/bitswell/loom-tools/src/tools/dag-check.ts` | no changes |
| `loom/.../references/protocol.md` | add §7.2 "Stack projection recipe" |
| `loom/.../references/schemas.md` | no changes |
| `loom/.../references/worker-template.md` | no changes |
| `loom/.../skills/loom/SKILL.md` | add one paragraph pointer to §7.2 |
| `loom-dispatch --scan` | one-line exclusion of `stack/*` |

Total net-new TypeScript: one file. Total net-new docs: two sections. Total protocol surface growth: zero.

---

## 4. Branch naming and scope

### 4.1 Two namespaces, one rule

LOOM owns `loom/<agent>-<slug>`. The stack projector owns `stack/<epic-slug>/<layer-n>`. They do not overlap, and the rule is: *nothing in `stack/*` is ever the source of truth for anything*.

```
loom/ratchet-auth-middleware   ← canonical, scope-enforced, agent-authored
loom/moss-api-endpoints        ← canonical, scope-enforced, agent-authored
loom/ratchet-frontend          ← canonical, scope-enforced, agent-authored

stack/add-auth/01-auth         ← mirror, no scope, orchestrator-only, disposable
stack/add-auth/02-api          ← mirror, no scope, orchestrator-only, disposable
stack/add-auth/03-frontend     ← mirror, no scope, orchestrator-only, disposable
```

The `loom/*` branches remain exactly as they are today: one agent per branch, one ASSIGNED commit from bitswell, `Scope:` enforced at integration, `--no-ff` merge into the workspace. Nothing about stacking touches them.

The `stack/*` branches are generated from scratch on every projection. They never receive a human commit. They never receive an agent commit. They exist only to hold the adopted state that `gh stack init --adopt` points at.

### 4.2 Naming details

- `<epic-slug>` is the assignment slug for the parent epic (the thing that spawned the agents). Pattern: kebab-case, same constraints as LOOM slugs.
- `<layer-n>` is a zero-padded 2-digit index: `01`, `02`, ... `NN`. Padding keeps branches lexically ordered, which makes `git branch --list 'stack/add-auth/*'` read naturally.
- Optionally (convention, not requirement): `stack/<epic-slug>/<NN>-<agent>-<slug>` so a reviewer glancing at the branch list can read the story.

Total length is bounded by the same 63-char git branch limit. The `stack/` prefix costs 6 characters; the `<NN>-` prefix costs 3; the remaining 54 are almost always enough for `<epic>/<agent>-<slug>`. If not, the projector truncates the `<agent>-<slug>` portion deterministically and logs a warning — it is a display-only branch, so truncation is safe.

### 4.3 `Scope:` across a stack

This is the question the RFP asks directly. Our answer: **there is no cross-stack `Scope:` question, because the stack is not the unit of scope enforcement.**

- `Scope:` is enforced at integration time, against the workspace, per `loom/*` branch, exactly as today.
- Mirror branches have no `Scope:` because they hold no original work — every commit they point at has already passed `Scope:` validation on its source `loom/*` branch.
- When a reviewer submits a change via the stack PR (e.g., "please rename this variable"), the orchestrator does not apply that change to the mirror branch. It re-dispatches the original worker on its original `loom/*` branch, the worker commits within its original `Scope:`, the workspace re-integrates, and `stack-project` re-runs. Scope is preserved by routing all authoring back through the canonical branch.

This is the invariant the RFP's section §3 is asking about, and the answer is: *Scope is enforced at integration, and integration still lives on `loom/*` only.*

### 4.4 Mirror namespace lifecycle

- **Creation:** `stack-project` creates or force-updates `stack/<epic-slug>/*` branches. If a branch with the same name exists from a prior run, it is force-updated (safe, because the branches are regenerated).
- **Mutation:** never mutated in place by anything other than `stack-project`. Workers are forbidden from committing to `stack/*` via the same mechanism that forbids them from committing to `main` — they do not have a worktree pointing at those branches.
- **Deletion:** on epic merge, the orchestrator runs `gh stack unstack` (which tears down local + GitHub state for the stack) and deletes local `stack/<epic-slug>/*` branches.
- **Garbage:** if an epic is abandoned, the mirror namespace is orphaned; a periodic `loom-dispatch` scan can sweep any `stack/*` branch older than N days that has no open PRs. This is a *nice-to-have*, not a requirement.

### 4.5 `loom-dispatch` exclusion

The one-line change to `loom-dispatch --scan` is:

```ts
const branches = allBranches
  .filter((b) => b.startsWith('loom/'))
  .filter((b) => !b.startsWith('stack/'));  // NEW: never scan mirror branches
```

(The second filter is redundant if the first already excludes `stack/`, but is added as a defense-in-depth line. The intent is explicit: `stack/*` is not dispatch territory.)

---

## 5. Merge vs rebase

This is the sharpest of the sharp edges. We answer it directly.

### 5.1 The conflict, stated plainly

- LOOM integrates agent work via `--no-ff` merges into the workspace. Every integration is a first-parent merge commit carrying the `Task-Status: COMPLETED` trailer set from the agent's final commit. `git log --first-parent` is the audit trail.
- `gh-stack` rebases. Every `gh stack rebase` or `gh stack sync` rewrites the stack branches and force-pushes. There is no notion of a "merge commit" for a stack layer — the `gh stack view --json` output has a `head` field, not a merge SHA.

If both of these are the source of truth, they contradict each other. One of them has to give.

### 5.2 How this proposal reconciles them

It reconciles them by **confining each mechanism to its own namespace**:

| Mechanism | Namespace | Semantics |
|---|---|---|
| `--no-ff` merge | workspace + `loom/*` | canonical history, audit trail, integration events |
| `gh stack rebase` / force-push | `stack/*` | disposable view, regenerated on every projection |

Because `stack/*` is disposable, force-pushing it is safe *by definition*: there is no history to destroy there, and no first-parent audit log to break. The authoritative audit trail is untouched.

Concretely: when a reviewer opens `stack/add-auth/02-api` and approves it, the orchestrator **does not merge that PR**. Merging a mirror PR would be a lie: the workspace merge for `loom/moss-api-endpoints` has already happened, and the mirror is just a view of it. Instead, the orchestrator:

1. Records the approval as an out-of-band signal (a PR label, a GitHub review submission).
2. On *final* approval of all layers, closes the stack PRs.
3. Runs the existing per-`loom/*` integration path (which may be: already-done, or open-final-PRs-against-main, depending on the epic's policy).
4. Tears down `stack/<epic-slug>/*`.

The audit trail lives on the `loom/*` branches and their `--no-ff` merges into the workspace. It is exactly the audit trail LOOM has today. It is not an amended, cherry-picked, or rebased version of the audit trail. It is the original.

### 5.3 "But the stack PR state machine breaks"

A fair objection: `gh stack view --json` will show the mirror PRs as OPEN forever from its point of view; it will never report MERGED. That is true, and it is the consequence we accept. Reviewers read `gh stack view` for their epic and see: "all open, one draft per layer." When the epic is done, the orchestrator closes the stack PRs via `gh pr close` and tears down the mirror branches. `gh stack` will then report the stack as non-existent, which is correct.

The "stack merged" state is a concept owned by `gh-stack`. We do not use it. We use LOOM's existing merge machinery.

### 5.4 What we do *not* do

We do not:

- Replace `--no-ff` integration with `gh stack sync`. That would destroy the audit trail and make cross-agent rebase conflicts load-bearing. LOOM's debuggability story depends on a mostly-linear-mergy workspace.
- Squash-merge stack PRs at the mirror level. This would create a fake merge commit on `main` for a layer that was already merged via `loom/*`.
- Cherry-pick from the mirror back into `loom/*`. That would rewrite agent branches after COMPLETED, violating the terminal-state rule in `protocol.md` §2.
- Teach `stack-project` to modify `loom/*` branches in any way. It only *reads* them.

### 5.5 Idempotence

The projection is **idempotent**. Running `stack-project` twice in a row, with the same arguments, converges on the same mirror state. That property comes for free because:

- Mirror branches are force-updated by SHA from the workspace's first-parent merge commits.
- `gh stack init --adopt` is idempotent on adopted branches (it re-adopts the current heads).
- `gh stack submit --auto --draft` syncs existing PRs rather than duplicating them.

Idempotence is what makes this proposal tractable: if an upstream change lands (a new worker retry, a hotfix), the recipe just re-runs. No state machine on the mirror side to worry about.

### 5.6 Freshness check

The one thing idempotence does *not* give us is a guarantee that the mirror is fresh. If a worker commits to `loom/ratchet-auth-middleware` after the projection runs, the stack PR on `stack/add-auth/01-auth` is a lie until the projector runs again.

To prevent silent drift, `stack-project` includes a **freshness check**: before submitting, it compares each `loom/*` branch's HEAD to the last source SHA embedded in the mirror branch's reflog (or, more simply, a `git notes` blob attached to the mirror commit). If any `loom/*` head has advanced past what the mirror was built from, `stack-project` fails with a clear message: "stale mirror; re-run projection." The orchestrator can then re-run, unattended, because the recipe is idempotent.

---

## 6. Worker authority

### 6.1 Workers invoke zero `gh stack` commands

This is the strongest invariant in the proposal. Workers:

- Do not run `gh stack init`.
- Do not run `gh stack add`.
- Do not run `gh stack submit`.
- Do not run `gh stack push`, `sync`, `rebase`, or `view`.
- Do not run `gh pr create` or `gh pr edit`. (This is the existing LOOM invariant; we preserve it.)
- Do not have any awareness that stacks exist. Their task spec is identical to today.

Only `bitswell` — the orchestrator — invokes `stack-project`, and only from the workspace. This is a natural extension of the existing rule "only bitswell writes to the workspace," which we restate as: "only bitswell publishes stacks."

### 6.2 LOOM invariants affected

Reading `protocol.md` §6.1 "Trust boundary":

| Boundary | State after this proposal |
|---|---|
| Workspace write | Unchanged: only orchestrator writes workspace |
| Agent scope | Unchanged: `Scope:` enforced on `loom/*` at integration |
| Cross-agent isolation | Unchanged: agents never touch other agents' worktrees |
| Prompt injection | Unchanged: no new prompt surfaces |

Zero invariants relaxed. Zero new trust boundaries introduced. The only change is that the orchestrator now has one more thing it is allowed to do — publish a stack view — and that new power is confined to a namespace the workers cannot see.

### 6.3 Why this matters

Most `gh-stack` integrations you will find on the internet assume the author of a branch runs the stack commands. That assumption is load-bearing for those integrations: it is how they handle branch-owner identity, force-push safety, and conflict resolution. Porting that assumption into LOOM is where most of the blast radius lives:

- Workers with `gh pr` authority can create PRs outside of `Scope:` checks (the check runs at integration, not at PR creation).
- Workers with `gh stack` authority can force-push anything on the stack, including parent layers, which breaks cross-agent isolation.
- Workers with GitHub credentials are a supply-chain surface. LOOM currently keeps that surface off the worker side entirely.

The read-only projection model lets us keep all of this tightly held in the orchestrator role. We do not relax it because relaxing it is not worth a UX improvement.

### 6.4 The corollary

If a future proposal wants to *extend* this one (say: adding a "worker publishes to stack" mode for a specific category of fast, low-risk work), it can layer that on top of `stack-project` without the base case being at risk. The minimal base case in this proposal keeps the door open; it does not slam shut.

---

## 7. End-to-end example

### 7.1 The epic

A three-agent epic: add authenticated user endpoints to a web app.

- Agent A: `ratchet` on `auth-middleware` — writes `server/middleware/auth.ts` and test.
- Agent B: `moss` on `api-endpoints` — writes `server/routes/users.ts` using the middleware. Depends on `ratchet/auth-middleware`.
- Agent C: `ratchet` on `frontend` — writes `web/src/pages/profile.tsx` calling the new endpoints. Depends on `moss/api-endpoints`.

Epic slug: `add-auth`.

### 7.2 Phase 1 — decomposition

The orchestrator (`bitswell`), in the workspace, drafts three `task(...)` commits on three new branches.

```
branch: loom/ratchet-auth-middleware
commit:
task(ratchet): add auth middleware with JWT verification

Wire a JWT-verification middleware in front of the API. Reject
missing/invalid tokens with 401.

Agent-Id: bitswell
Session-Id: 8f4a...-b1c3
Task-Status: ASSIGNED
Assigned-To: ratchet
Assignment: auth-middleware
Scope: server/middleware/**, server/middleware/**.test.ts
Dependencies: none
Budget: 40000
```

```
branch: loom/moss-api-endpoints
commit:
task(moss): add user API endpoints

Implement GET /api/users/me and PATCH /api/users/me behind the
auth middleware.

Agent-Id: bitswell
Session-Id: 8f4a...-b1c3
Task-Status: ASSIGNED
Assigned-To: moss
Assignment: api-endpoints
Scope: server/routes/users.ts, server/routes/users.test.ts
Dependencies: ratchet/auth-middleware
Budget: 40000
```

```
branch: loom/ratchet-frontend
commit:
task(ratchet): add profile page

Build a profile page that fetches GET /api/users/me and renders it.

Agent-Id: bitswell
Session-Id: 8f4a...-b1c3
Task-Status: ASSIGNED
Assigned-To: ratchet
Assignment: frontend
Scope: web/src/pages/profile.tsx, web/src/pages/profile.test.tsx
Dependencies: moss/api-endpoints
Budget: 40000
```

Before spawning, the orchestrator validates the DAG:

```
dag-check {
  agents: [
    { id: 'ratchet/auth-middleware', dependencies: [] },
    { id: 'moss/api-endpoints',      dependencies: ['ratchet/auth-middleware'] },
    { id: 'ratchet/frontend',        dependencies: ['moss/api-endpoints'] }
  ]
}
→ ok: true
  integrationOrder: [
    'ratchet/auth-middleware',
    'moss/api-endpoints',
    'ratchet/frontend'
  ]
```

That `integrationOrder` will be reused twice: once as the workspace merge order, and once as `stack-project`'s `order` input. This is the key reuse of existing primitive.

### 7.3 Phase 2 — parallel execution

The orchestrator spawns all three workers in parallel worktrees (ratchet-A and moss can run concurrently; ratchet-C is gated by its dependency). Each worker:

1. Commits `chore(...): begin ...` with `Task-Status: IMPLEMENTING`.
2. Writes code inside its `Scope`.
3. Commits `feat(...)` with `Task-Status: COMPLETED`, `Files-Changed`, `Key-Finding`.

After the phase:

```
loom/ratchet-auth-middleware   HEAD: feat(auth): add JWT middleware (COMPLETED)
loom/moss-api-endpoints        HEAD: feat(api): add /api/users/me (COMPLETED)
loom/ratchet-frontend          HEAD: feat(web): add profile page (COMPLETED)
```

Workers have not touched `gh`, `gh stack`, `gh pr`, or any remote. They have not seen the string `stack`.

### 7.4 Phase 3 — integration

The orchestrator merges the three branches into the workspace in DAG order, via `--no-ff`, one at a time, running per-branch `Scope:` checks before each merge:

```
git checkout workspace
git merge --no-ff --no-edit loom/ratchet-auth-middleware
  → merge commit M1 on workspace
git merge --no-ff --no-edit loom/moss-api-endpoints
  → merge commit M2 on workspace
git merge --no-ff --no-edit loom/ratchet-frontend
  → merge commit M3 on workspace
```

After this phase, `git log --first-parent workspace` reads:

```
M3  Merge loom/ratchet-frontend         [first-parent]
M2  Merge loom/moss-api-endpoints       [first-parent]
M1  Merge loom/ratchet-auth-middleware  [first-parent]
... previous epic ...
```

Each merge commit carries the `Task-Status: COMPLETED` of its source branch via the second parent's history. The audit trail is complete and identical to today.

### 7.5 Phase 4 — projection

Now the new part. The orchestrator calls `stack-project` from the workspace:

```
stack-project {
  epic: 'add-auth',
  order: [
    'ratchet/auth-middleware',
    'moss/api-endpoints',
    'ratchet/frontend'
  ],
  base: 'main',
  draft: true
}
```

The tool executes roughly:

```bash
# 1. Create or force-update mirror branches from workspace merge commits
git branch -f stack/add-auth/01-ratchet-auth-middleware <M1>
git branch -f stack/add-auth/02-moss-api-endpoints     <M2>
git branch -f stack/add-auth/03-ratchet-frontend       <M3>

# 2. Adopt them into a stack
gh stack init --base main --adopt \
  stack/add-auth/01-ratchet-auth-middleware \
  stack/add-auth/02-moss-api-endpoints \
  stack/add-auth/03-ratchet-frontend

# 3. Push and create draft PRs
gh stack submit --auto --draft

# 4. Inspect and return
gh stack view --json
```

The output, simplified:

```json
{
  "trunk": "main",
  "currentBranch": "stack/add-auth/03-ratchet-frontend",
  "branches": [
    { "name": "stack/add-auth/01-ratchet-auth-middleware",
      "pr": { "number": 201, "state": "OPEN" } },
    { "name": "stack/add-auth/02-moss-api-endpoints",
      "pr": { "number": 202, "state": "OPEN" } },
    { "name": "stack/add-auth/03-ratchet-frontend",
      "pr": { "number": 203, "state": "OPEN" } }
  ]
}
```

Reviewers see a three-layer stack of draft PRs. Each PR's body starts with:

```
[MIRROR] This is a projected view of a LOOM epic. Approving this
PR is a review signal only; the canonical branch lives at
loom/ratchet-auth-middleware and has already been integrated.
Requested changes flow back through that branch via a re-dispatch.
```

### 7.6 Phase 5 — review feedback

A reviewer on PR #202 (`stack/add-auth/02-moss-api-endpoints`) asks: "The PATCH endpoint should check Content-Length."

The orchestrator does *not* edit `stack/add-auth/02-moss-api-endpoints`. It:

1. Creates a follow-up assignment commit on `loom/moss-api-endpoints` with `Task-Status: ASSIGNED` and a new slug (or reuses the existing branch if the re-dispatch policy allows amendments to a non-terminal branch state — per protocol.md §2, terminal-state rules apply, so in practice this is a new worker on a new branch like `loom/moss-api-endpoints-v2` that supersedes the original).
2. Runs the worker, gets a new COMPLETED commit, integrates with `--no-ff` into the workspace, producing merge commit M2'.
3. Re-runs `stack-project`. The projector detects that the workspace merge commit for `moss/api-endpoints` has moved from M2 to M2', force-updates `stack/add-auth/02-moss-api-endpoints` to point at M2', and `gh stack submit --auto --draft` syncs the PR. Layers 03 (frontend) may need to be rebased on top; `gh stack rebase --upstack` handles this because the mirror branches are safe to rebase (no scope, no authorship, no audit meaning).

PR #202 updates in place. Reviewer sees the new diff, approves.

### 7.7 Phase 6 — final approval and teardown

When all three stack PRs are approved, the orchestrator:

1. Closes stack PRs #201, #202, #203 with `gh pr close` and a comment linking to the canonical `loom/*` branches.
2. Runs `gh stack unstack` to tear down the local + remote stack state.
3. Deletes `stack/add-auth/01-*`, `stack/add-auth/02-*`, `stack/add-auth/03-*` branches.
4. Opens the final integration PR(s) against `main`. This step is governed by the existing LOOM policy: it might be one `--no-ff` merge PR for the entire epic, or three per-`loom/*` PRs, depending on repo convention. Either way, the authoritative merge is against `main` via `pr-create`, not via `gh stack`.
5. On merge, the workspace's epic is closed, and the audit trail on `main` is the usual first-parent merge story.

From the `loom/*` side, the audit trail reads: three ASSIGNED commits, three IMPLEMENTING-through-COMPLETED agent histories, three first-parent merges into workspace, one (or three) PR merges into `main`. Identical to today.

From the `stack/*` side: three branches were created, three draft PRs were opened, one layer was force-updated after review feedback, all three were closed on final approval, all three branches were pruned. The mirror left behind no commits on `main`, no trailer mutations, no audit-trail changes.

### 7.8 Total tool calls

For the whole epic, the orchestrator made these tool calls in this order:

1. `dag-check` (plan-gate)
2. worker spawns ×3 (parallel where allowed)
3. workspace merges ×3
4. `stack-project` ×1 (initial projection)
5. (after review feedback) re-dispatch one worker
6. workspace merge ×1 (the fix)
7. `stack-project` ×1 (re-project)
8. `gh pr close` ×3 (teardown)
9. `gh stack unstack` ×1 (teardown)
10. `pr-create` ×1 or ×3 (final integration PRs)

Every step except the two `stack-project` calls already exists today. The new code exercised on the happy path is one tool, called twice.

---

## 8. Risks and rejected alternatives

### 8.1 Risks (ordered by how much they cost)

**Risk 1: reviewer confusion over approval semantics.**
Mirror PRs do not merge. A reviewer clicks "Approve" and nothing happens, which is surprising. A reviewer clicks "Merge" and either gets blocked by branch protection or merges a mirror branch into main (which is bad).

*Mitigation:* three layers of defense. (a) Mirror branches are protected at the GitHub level so that "merge PR" is disabled for anything in `stack/*`. (b) PR title prefix `[MIRROR]` makes the status visible on every list view. (c) PR body template explains the projection model in the first two lines. A reviewer who misses all three is a reviewer who would have misread any proposal.

**Risk 2: silent mirror drift.**
A worker commits to `loom/moss-api-endpoints` after a projection. The stack PR for layer 02 is now showing yesterday's code, but the PR number has not changed and nothing on GitHub hints at staleness.

*Mitigation:* the freshness check in §5.6. `stack-project` attaches a reference to the source SHA it built each mirror from (either in the mirror's reflog, or as a `git notes` blob, or baked into a `Mirror-Source:` metadata section in the mirror PR body). Re-running the recipe is idempotent, so the cost of staying fresh is one recipe call. The orchestrator runs it on every upstream change to the epic. On top of that, we add a dispatch-time check: if `stack-project` detects that any `loom/*` HEAD is ahead of the tracked mirror source SHA, it either fails cleanly ("stale; re-run") or auto-re-projects (flag-controlled).

**Risk 3: cherry-pick conflicts when projecting.**
In the end-to-end example we skipped a subtlety: `stack-project` creates mirror branches by force-branching at workspace merge commits, but the merge commit on the workspace includes changes from *previous* layers by definition. If a reviewer opens PR #202 expecting to see only `moss/api-endpoints`'s diff, they will actually see the cumulative diff relative to trunk — which, because `gh-stack` sets each layer's base to the layer below, *is* the per-layer diff in practice. The subtlety is that if the workspace's first-parent graph is not cleanly staircased (e.g., because an orchestrator post-terminal hotfix landed in the middle), the per-layer diff the reviewer sees may not match what the worker actually wrote.

*Mitigation:* project in strict DAG order, reuse `git rerere` (which `gh stack init` enables automatically), and fail the projection cleanly rather than half-publishing. In the worst case, `stack-project` aborts and the epic falls back to the existing per-`loom/*` PR flow. The user loses the ladder UX for that epic; they do not lose correctness.

**Risk 4: `gh-stack` extension drift.**
`gh-stack` is a third-party extension under active development. A breaking change to `gh stack init --adopt` or `gh stack view --json` could break `stack-project`.

*Mitigation:* pin the `gh-stack` version in the orchestrator environment (install via `gh extension install github/gh-stack@<sha>`). The skill file at `/home/willem/.agents/skills/gh-stack/SKILL.md` is already the canonical reference in this repo; we add a `version:` field to the projection recipe and bump it explicitly.

**Risk 5: "no one uses the stacks."**
The projection model imposes a small cognitive load on reviewers, and if the UX improvement does not pay off — if reviewers continue to open the `loom/*` PRs anyway — we have spent one tool and one recipe on nothing.

*Mitigation:* the fact that we can delete `stack-project.ts` and the recipe with zero workspace impact. This proposal is deliberately reversible. "Is this a good idea?" is a question the `stack-project` tool is itself a test for. If adoption is low, remove it; nothing downstream breaks.

### 8.2 Rejected alternative 1 — "give workers `gh stack` authority"

The fastest conceptual path to real stacks is to let each worker push its own layer. The worker that holds `loom/moss-api-endpoints` just runs `gh stack add api-endpoints` on top of whatever the dependency layer is, and reviewers get actual merge-able stacked PRs.

**Why rejected:**
This breaks the "only orchestrator touches GitHub PRs" invariant in `protocol.md` §6.1. Workers with PR-creation authority bypass `Scope:` enforcement (which runs at integration, not at PR creation), can force-push their own layer across a rebase, and introduce a new credential surface on the worker side. The audit trail fragments across rebased branches: a worker can amend and force-push after its own COMPLETED commit, which violates the terminal-state rule. And, critically, the rebase-based audit trail is not the audit trail we want — the merge-based one is more debuggable.

It is fast. It is also exactly the proposal that destroys the invariants the RFP's sharp-edge #5 flags as expensive. Not worth the blast radius.

### 8.3 Rejected alternative 2 — "replace `--no-ff` integration with `gh stack sync`"

Treat `gh-stack` as the integration mechanism. LOOM's workspace gets replaced by the stack bottom; integration events become `gh stack rebase`; `gh stack sync` is the new `--no-ff` merge.

**Why rejected:**
This would give stacks first-class status in LOOM, but it destroys the merge-based audit log. Post-mortem queries like `git log --first-parent workspace` no longer work. Cross-agent rebase conflicts become load-bearing — every rebase on a stack layer can fail for reasons unrelated to that layer's author, and the orchestrator has no clean way to attribute the failure. LOOM's debuggability story depends on a mostly-linear-mergy workspace history where each merge is labeled with the agent that produced it. A rebased stack history does not have that property.

The rejection is on invariants, not taste. `gh stack sync` is great for a team of humans. It is the wrong substrate for a multi-agent orchestrator that must answer "which agent wrote this blob?" from the git log directly.

### 8.4 Rejected alternative 3 — "new `Stack-Parent:` / `Stack-Order:` trailers"

Extend the trailer vocabulary with explicit stack metadata. Each worker commits `Stack-Parent: moss/api-endpoints` and `Stack-Order: 2` on its COMPLETED commit. The orchestrator reads these at integration time and drives the projection from the trailers instead of from `dag-check`.

**Why rejected:**
It adds protocol surface area without removing any existing surface. The `Dependencies:` trailer *already* encodes order via `dag-check`'s topological sort. Adding a second way to encode the same information is a schema smell; it creates a consistency question (what if `Dependencies:` and `Stack-Parent:` disagree?) and it forces the worker template to care about stacking.

The trailer vocabulary is load-bearing. We do not grow it unless a primitive is missing. In this case the primitive is present, and the rejection is on parsimony.

### 8.5 Rejected alternative 4 — "store the mirror in a separate git repo"

Push `stack/*` to a separate repository rather than coexist in the same repo as `loom/*` and `main`. Reviewers see a clean, purpose-built "review repo" with no `loom/*` noise.

**Why rejected:**
Two repositories is a coordination overhead we cannot justify. The mirror repo would need its own auth, its own webhook routing back to the source repo, its own PR-comment syncing, and its own CI. The review experience gets *worse*, not better, because reviewers now have to navigate across two URLs to see source and discussion together. And the freshness-check machinery becomes cross-repo, which is materially harder than intra-repo.

The single-repo, separate-namespace approach gets the same isolation benefit (reviewers can filter PR lists by `stack/*` or by `loom/*`) at a fraction of the cost.

### 8.6 Rejected alternative 5 — "use `gh stack init --adopt` on `loom/*` directly"

Instead of creating a mirror namespace, just adopt the `loom/*` branches themselves into a stack. Keep one namespace; let `gh-stack` manage it.

**Why rejected:**
This is the alternative that looks the simplest on paper and is the most dangerous in practice. `gh stack init --adopt` means the adopted branches become subject to `gh stack rebase` and force-push. `loom/*` branches are agent-authored, scope-enforced, lifecycle-tracked artifacts; force-pushing them erases `Task-Status` history, rewrites the terminal COMPLETED commit, and turns the audit trail into a lie. It also means `gh stack submit` would try to open a PR on every `loom/*` branch, including BLOCKED and FAILED ones, which is semantically wrong.

In short: the `loom/*` namespace is the wrong substrate for `gh-stack` because `gh-stack` assumes mutable branches and `loom/*` assumes append-only. The mirror namespace exists precisely to bridge that mismatch.

---

## 9. Closing

The bet in this proposal is that the "sharp edges" in the RFP are not inherent to adopting `gh-stack` — they are inherent to adopting `gh-stack` *as the integration mechanism*. By treating `gh-stack` as an output lens, five of the sharp edges go away, two turn into reusable primitives (DAG + custom PR bases), and one (merge vs rebase) resolves cleanly on a namespace boundary.

The cost is measured: one new tool, one new recipe, zero new trailers, zero new worker surface, zero changes to scope enforcement, zero changes to the audit trail. The escape hatch is free: delete the tool and the recipe and LOOM is exactly where it is today.

The UX win is real: reviewers get a ladder of draft PRs for every epic, each PR showing the diff of one layer on top of the previous, matching the dependency DAG the orchestrator already validates. The cost to reviewers is a cognitive tax (the mirrors don't merge) mitigated by title prefixes, body templates, and branch-level protection.

If another proposal lands that wants real stacked integration, it can build on this one — the projection layer is the minimum viable case, not a dead-end. If this is as far as the protocol ever goes, LOOM still got the UX it wanted without spending any of the invariants it cannot afford to lose.
