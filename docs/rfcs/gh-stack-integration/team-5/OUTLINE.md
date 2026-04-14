# Team 5 — OUTLINE

## Angle statement

For epics opted in via a `Stack-Mode: true` trailer on the root ASSIGNED
commit, the orchestrator SKIPS `--no-ff` merge integration entirely and lands
the epic through `gh stack rebase` + `gh stack submit` onto a linear rebased
history; the stack itself becomes the audit trail, replacing the merge-commit
trail for that epic.

## Thesis

LOOM's `--no-ff` integration buys one thing that matters: `git log
--first-parent main` tells you exactly which epic each change belongs to, and
the merge commit is a stable provenance anchor the orchestrator signs. That
is worth a lot for long-running, multi-week epics where provenance queries
outlive the work. It is worth much less for a 3-agent feature epic where the
dependency chain itself *is* the story and the reviewer's top complaint is
that they cannot see the layers. In that second regime, the ladder UX of
stacked PRs and the linear rebased history are strictly more valuable than a
merge-commit audit trail that nobody reads because the epic is done in an
afternoon. Opt-in at the epic level is the right granularity because it
mirrors how the tradeoff actually moves: per-epic, not per-repo and not
per-agent. Stack-mode epics pay the cost of a lossier provenance model
(commit metadata, not merge topology) in exchange for reviewability, and
merge-mode epics stay exactly as they are. Epics choose at assignment time
and cannot mix within one epic.

The question this proposal must answer — when are linear rebased histories
preferable to `--no-ff` merge audit trails, and what do we give up? — has a
clean answer under this angle: *when the epic is small-to-medium, the
dependency chain is the review story, and nobody will query
`--first-parent main` for its provenance six months later*. What we give up
is the stable first-parent projection, the pre-integration diff record of
each agent branch (workers still keep pre-rebase reflogs briefly, but the
integrated history shows only the rebased commits), and the ability to
`git revert -m 1 <merge-sha>` the whole epic as one atom. We reconstitute
the audit trail from per-commit `Agent-Id`, `Session-Id`, `Epic-Id`, and
`Task-Status` trailers, which are already required on every commit, plus one
new `Epic-Id` trailer stamped on every commit the orchestrator emits during
`gh stack rebase`.

## Section headers for proposal.md

### 1. Angle statement

- Restate in one sentence: stack-mode replaces `--no-ff` integration for
  opted-in epics; for those epics the `gh stack` linear history is the
  canonical and only integration record.
- Name the opt-in mechanism: `Stack-Mode: true` trailer on the epic's root
  ASSIGNED commit (the one that declares the DAG), mirrored into
  `AGENT.json` under `epic.stack_mode: true` for tooling that queries
  without parsing commits.
- State the scope explicitly: the choice is per-epic, not per-agent and not
  per-repo. Mixing integration modes inside one epic is forbidden.
- Name what is replaced, not augmented: the `pr-merge` call with
  `--method merge`, the workspace `git merge --no-ff`, and the first-parent
  audit projection — all three are skipped for stack-mode epics.
- Assert the invariant: for a stack-mode epic, no merge commit touching
  agent branches ever appears on `main`. The only commits on `main` are the
  rebased worker commits, in dependency order.

### 2. What changes

- **`loom-tools` — `pr-merge.ts`**: grows a branch. When the epic's
  `Stack-Mode` is true, `pr-merge` refuses to run and returns
  `err('stack-mode-epic', 'use stack-submit', false)`. Callers (the
  orchestrator integration recipe) MUST route through a new tool instead.
- **`loom-tools` — new `stack-submit.ts`**: an orchestrator-only tool that
  runs `gh stack rebase` then `gh stack submit --auto` in the epic's
  dedicated working tree. It is the stack-mode replacement for `pr-merge`.
  Inputs: `epicId`, `branches[]` (already in topo order from `dag-check`),
  `remote`. Outputs: ordered list of `{branch, sha, prNumber}`.
- **`loom-tools` — new `stack-land.ts`**: the stack-mode replacement for the
  workspace `git merge --no-ff` step. It runs `gh stack sync` (which
  rebases, force-pushes, and detects squash-merged PRs), then waits for all
  stack PRs to hit `MERGED` state via `gh stack view --json`, and finally
  fast-forwards `main`. No merge commit is produced.
- **`loom-tools` — `dag-check.ts`**: unchanged in its core logic but
  `scope-check.ts` learns a new relaxation: in stack-mode epics, `Scope`
  trailers are still enforced, but the integration-time scope check is run
  against the *rebased* diff, not the original branch diff, since rebase
  can drop or re-order hunks.
- **`loom-tools` — `commit.ts` / `trailer-validate.ts`**: add `Stack-Mode`
  and `Epic-Id` to the known trailer vocabulary. `Stack-Mode` is allowed
  only on the epic's root ASSIGNED commit. `Epic-Id` becomes required on
  every commit inside a stack-mode epic (so the rebased history can be
  queried by epic).
- **`loom` plugin/skill — integration recipe**: the orchestrator's
  `integrate-epic` recipe gains a branch at the top: if the epic's root
  commit has `Stack-Mode: true`, dispatch to `stack-submit` + `stack-land`
  and skip `pr-merge` and the `git merge --no-ff` step entirely. Non-stack
  epics take the untouched existing path.
- **Worker template**: a single added line. Workers in stack-mode epics
  MUST stamp `Epic-Id: <epic-slug>` on every commit. Nothing else changes —
  workers still never call `gh stack` themselves.
- **Schemas**: three additions — `Stack-Mode` (boolean, optional, root-only),
  `Epic-Id` (string, required in stack-mode), and a new terminal-equivalent
  integration outcome documented in §5.7 for stack-mode: the orchestrator's
  post-terminal commit is the rebased stack sync, not a merge commit.

### 3. Branch naming and scope

- Worker branches keep the `loom/<agent>-<slug>` convention unchanged. Scope
  trailers on workers are unchanged and enforced at the first pre-rebase
  check (identical to today).
- The orchestrator constructs the stack on a `stack/<epic-slug>/*`
  namespace, reusing each worker's branch content via
  `gh stack init --base main --adopt loom/<a1>-<s1> loom/<a2>-<s2> ...` in
  topological order. The adopted branches become the layers of the stack;
  no new branches are created.
- `gh stack`'s own `-p` prefix is left unset for stack-mode epics because
  adopting pre-existing LOOM branches requires passing full branch names,
  and the `loom/` prefix already provides the namespace.
- Scope enforcement post-rebase: because `gh stack rebase` replays commits,
  the integration-time scope check runs `git diff --name-only main...<layer>`
  *after* rebase and verifies every path still matches the worker's
  declared `Scope`. If a rebase silently lost a file (it cannot, without
  conflict, but we check anyway), the stack-mode integration fails closed.
- Branch cleanup: after all stack PRs are `MERGED`, the `loom/*` branches
  are kept for 30 days per §5.2 of protocol.md, but the orchestrator deletes
  the stack's local tracking via `gh stack unstack --local` so the same
  branches can be re-adopted by a later retry if needed.

### 4. Merge vs rebase

**This is the core section. The linear rebased history must carry, by
construction, every piece of information the `--no-ff` merge trail carried
for a stack-mode epic — otherwise the angle fails.**

- **What `--no-ff` carried**: (a) epic grouping via `--first-parent main`,
  (b) the orchestrator's signature on the merge commit as the integration
  anchor, (c) the pre-rebase form of each worker branch as a second parent,
  (d) atomic revertability via `git revert -m 1 <merge-sha>`, (e) a stable
  merge SHA that downstream consumers (CI, release notes, dashboards) key
  off. Each must be replaced or consciously dropped.
- **Replacement (a), epic grouping**: every commit in a stack-mode epic
  carries `Epic-Id: <epic-slug>`. `git log --grep='Epic-Id: auth-epic'` or
  `git log --format='%(trailers:key=Epic-Id,valueonly)'` reconstitutes the
  same grouping the merge commit used to provide. This is strictly more
  queryable than first-parent (it survives rebases, squashes, and
  cherry-picks) but loses the topological "this is one atom" hint.
- **Replacement (b), integration anchor**: the orchestrator's signature
  moves from the merge commit to an annotated, signed tag
  `stack-landed/<epic-slug>` pointing at the top of the landed stack. The
  tag message contains the integration manifest (branches, SHAs, PR
  numbers, timestamp, orchestrator session-id). This is the audit-trail
  replacement: one artifact per epic, signed by the orchestrator, queryable
  via `git for-each-ref refs/tags/stack-landed/*`.
- **Drop (c), pre-rebase branch form**: this is the real loss. Once
  `gh stack rebase` runs, the pre-rebase SHAs survive only in reflog
  (local, expires) and the `loom/*` branches (retained 30 days per
  protocol). After 30 days the pre-rebase form is gone. We accept this.
  Mitigation: for epics that need long-term pre-rebase provenance, do not
  opt into stack-mode.
- **Drop (d), atomic revert**: `git revert -m 1 <merge-sha>` has no
  stack-mode equivalent. Reverting a stack-mode epic means reverting each
  of its commits in reverse order, which `gh pr revert` does not automate
  at stack granularity. Mitigation: the `stack-landed/<epic-slug>` tag
  enumerates exactly which commits to revert, and a new
  `loom-tools/stack-revert.ts` helper can read the tag and produce the
  revert sequence. This is a non-trivial loss for operations.
- **Replacement (e), stable integration SHA**: downstream consumers key off
  the `stack-landed/<epic-slug>` tag SHA instead of a merge SHA. The tag
  is stable once landed. CI / release-notes / dashboards gain one grep
  pattern and lose one.
- **What breaks for audit consumers**: (1) any tool that does
  `git log --first-parent main` to list epics will miss stack-mode epics
  or see their individual commits interleaved with other first-parents —
  we supply a shim that unions first-parent with
  `git tag --list 'stack-landed/*'`. (2) any dashboard that counts merge
  commits undercount stack-mode work — we document this and recommend
  migrating to `Epic-Id` counting. (3) `git blame` on stack-mode code points
  at the rebased worker commit, not a merge commit, which is actually an
  improvement for debugging but a change for any scripts that assumed
  merge-commit blame.
- **Why linear is preferred in stack-mode**: reviewers reading a stack-mode
  epic read commits as-rebased in dependency order with no merge-commit
  noise; `git log` is the review ladder. Reviewers reading a merge-mode
  epic see `git log --first-parent main` as a terse list of merge commits
  and must drill into second-parents to read code. For the target regime
  (small-to-medium feature epics), the former is strictly better.

### 5. Worker authority

- Workers still never invoke `gh stack` commands. Not in stack-mode, not in
  merge-mode. The invariant is preserved.
- Only the orchestrator runs `gh stack init --adopt`, `gh stack rebase`,
  `gh stack submit --auto`, `gh stack sync`, and `gh stack unstack`. It does
  so from a dedicated epic-integration working tree that only the
  orchestrator writes to.
- The one new worker obligation in stack-mode: stamp `Epic-Id: <epic-slug>`
  on every commit. This is a trailer addition, not a new tool call. It is
  validated by `trailer-validate.ts` at the same enforcement point as
  `Agent-Id` and `Session-Id`.
- The orchestrator's post-rebase scope check is a *new* authority
  boundary: after `gh stack rebase` rewrites commits, the orchestrator is
  the sole entity that verifies the rebased commits still match worker
  `Scope` trailers. Workers cannot verify this themselves because they
  never see the rebased form.
- Stack-mode conflict handling: if `gh stack rebase` hits a conflict, the
  orchestrator does NOT attempt to resolve — it emits a BLOCKED terminal
  commit on the stuck layer and re-dispatches a worker (new session) to
  the pre-rebase branch with a `Conflict-Resolution` task. This preserves
  the "only workers edit code" invariant through the rebase machinery.
- Explicitly reject: "let the orchestrator resolve rebase conflicts". It
  would violate workspace-write isolation by having the orchestrator edit
  source files, and it would hide the conflict resolution from the epic's
  commit history.

### 6. End-to-end example

**Side-by-side, same three agents, same tasks, different integration
modes.** The epic: auth middleware → API endpoints → frontend, three
agents (`ratchet`, `moss`, `drift`), dependencies `ratchet/auth-mw` →
`moss/api` → `drift/ui`.

#### 6.1 Merge-mode epic (unchanged — for contrast)

- **Decomposition**: orchestrator commits three ASSIGNED tasks on
  `loom/ratchet-auth-mw`, `loom/moss-api` (deps: `ratchet/auth-mw`),
  `loom/drift-ui` (deps: `moss/api`). No `Stack-Mode` trailer.
- **Work**: three workers commit, each reaches `Task-Status: COMPLETED`.
- **Integration**: orchestrator calls `pr-create` on each branch, `pr-merge
  --method merge` on PR1, retargets PR2 and PR3, merges PR2, retargets PR3,
  merges PR3. Workspace receives three `--no-ff` merge commits.
- **Final history on main** (from `git log --first-parent`):
  ```
  M3 Merge loom/drift-ui                (merge commit)
  M2 Merge loom/moss-api                (merge commit)
  M1 Merge loom/ratchet-auth-mw         (merge commit)
  ```
  Each merge has a second parent pointing at the worker's terminal commit.
- **Audit**: `git log --first-parent main --grep='Merge loom/'` lists the
  three merges. `git revert -m 1 M3` reverts the frontend atom.
- **Artifacts**: 3 PRs, 3 merge commits on main, worker branches retained.

#### 6.2 Stack-mode epic (new path, same tasks)

- **Decomposition**: orchestrator commits three ASSIGNED tasks identically,
  plus `Stack-Mode: true` and `Epic-Id: auth-epic` on the root task
  (`loom/ratchet-auth-mw`). Dependent tasks inherit `Epic-Id: auth-epic`.
- **Work**: three workers commit exactly as before, with `Epic-Id:
  auth-epic` on every commit. No worker knows or cares that this is
  stack-mode.
- **Integration (new)**: orchestrator runs:
  ```bash
  gh stack init --base main --adopt \
    loom/ratchet-auth-mw loom/moss-api loom/drift-ui
  gh stack rebase
  # scope-check.ts runs against rebased diffs; passes
  gh stack submit --auto --draft
  # three stacked PRs created, linked via gh stack's stack metadata
  # human reviewer approves each layer; marks PRs ready
  gh stack sync                  # picks up merges, rebases remainder
  # wait loop: gh stack view --json until all branches isMerged:true
  git tag -a -s stack-landed/auth-epic -m "<manifest>"
  git push origin stack-landed/auth-epic
  ```
- **Final history on main** (from `git log` — note NO `--first-parent`
  because there are no merges):
  ```
  C5 drift: frontend dashboard     Epic-Id: auth-epic, Agent-Id: drift
  C4 drift: begin frontend         Epic-Id: auth-epic, Agent-Id: drift
  C3 moss: API endpoints           Epic-Id: auth-epic, Agent-Id: moss
  C2 ratchet: auth middleware tests Epic-Id: auth-epic, Agent-Id: ratchet
  C1 ratchet: auth middleware      Epic-Id: auth-epic, Agent-Id: ratchet
  ```
  Plus an annotated tag `stack-landed/auth-epic` pointing at C5, signed
  by the orchestrator, with the manifest in the tag message.
- **Audit**: `git log --grep='Epic-Id: auth-epic'` lists all five commits.
  `git show stack-landed/auth-epic` shows the signed manifest. Reverting
  means `git revert C5 C4 C3 C2 C1` in reverse order (or the
  `stack-revert.ts` helper reads the tag and does it).
- **Artifacts**: 3 PRs (stacked, linked in GitHub), 0 merge commits on
  main, 1 signed tag, 5 rebased commits on main, worker branches retained.

#### 6.3 The contrast

- Merge mode: reviewers see 3 big PRs each targeting main; the ladder is
  implicit in `Dependencies` trailers but not visible in GitHub's UI.
  Audit is via first-parent merges and revert is atomic per epic.
- Stack mode: reviewers see a 3-PR ladder in GitHub's stacked-PR UI;
  each layer shows only its own diff. Audit is via `Epic-Id` grouping and
  a signed tag. Revert is per-commit or via helper.
- Both epics do the same work in the same order by the same workers. The
  only difference is the shape of the integration record.

### 7. Risks and rejected alternatives

- **Risk: an audit consumer we didn't know about keys off `--first-parent
  main`.** Mitigation: announce the stack-mode opt-in loudly, provide the
  shim that unions first-parent with `stack-landed/*` tags, keep stack-mode
  strictly opt-in so existing epics are unaffected.
- **Risk: `gh stack rebase` silently drops or re-orders changes in a way
  that passes scope-check but corrupts intent.** Mitigation: post-rebase
  scope-check runs `git diff` against the pre-rebase form (which still
  exists on the `loom/*` branch before the stack is submitted) and
  verifies the rebased tree is identical content-wise to the pre-rebase
  topo-merge. If not, fail closed.
- **Risk: stack-mode is chosen for an epic that later needs atomic revert.**
  Mitigation: `stack-revert.ts` helper, documented at opt-in time. Worst
  case: a human reverts commits one by one from the tag manifest.
- **Risk: `gh stack`'s strict linearity clashes with non-linear DAGs.**
  Mitigation: `scope-check.ts` refuses to enable stack-mode on an epic
  whose DAG has fan-out (`Dependencies` listing more than one parent).
  Fan-out epics MUST use merge mode. This is checked at the assignment
  gate, not at integration time.
- **Risk: conflict resolution during rebase violates worker-authority.**
  Mitigation: orchestrator never edits code during `gh stack rebase`;
  conflicts produce a BLOCKED terminal state and a new `Conflict-Resolution`
  worker assignment on the conflicting layer.
- **Rejected alternative 1: "convert merge-mode epics to stack-mode at
  integration time, after workers finish."** Sounds like free ladder UX
  for all epics, but requires the orchestrator to have already committed
  to the opt-in before workers finish (because `Epic-Id` trailers must
  be stamped by workers). Retrofitting `Epic-Id` across terminal commits
  means orchestrator post-terminal edits, which protocol §2 allows but
  which makes the trailer's meaning ambiguous ("was it really there at
  work time?"). Cleaner to decide at decomposition.
- **Rejected alternative 2: "keep `--no-ff` merges AND run `gh stack` as
  a parallel projection."** This is team 1's angle (post-integration
  projection) and it preserves the merge trail. We explicitly reject it
  for stack-mode because the whole point of stack-mode is that the
  rebased history is the canonical record; a parallel projection gives
  you neither clean history (merges remain) nor a single source of truth
  (two topologies). The angle gate rejected this as hedging, and rightly
  so.
- **Rejected alternative 3: "stack-mode at the agent level, not the epic
  level."** Lets individual agents within an epic opt in. Breaks because
  a mid-chain agent in stack-mode depending on a merge-mode parent would
  need its pre-rebase SHA to be an ancestor of main (it isn't — the
  parent merged as `--no-ff`), and `gh stack` would refuse to adopt it.
  Epic-level opt-in is the only coherent granularity.

## Key references

- `/home/willem/.agents/skills/gh-stack/SKILL.md` — `sync` (lines ~557-586),
  `rebase` (lines ~589-630), `submit --auto` (lines ~519-555),
  `init --adopt` (line ~420), `unstack --local` (lines ~737-763). The
  strict-linearity limitation at line ~789 is the hard constraint forcing
  the "no fan-out DAGs" rule in section 7.
- `/home/willem/.claude/plugins/cache/loom-plugin/loom/0.1.0/skills/loom/references/protocol.md`
  §3.3 (integration model — the exact path this proposal replaces for
  stack-mode), §6.1 (trust boundaries — preserved), §8.2 (audit trail —
  reconstituted via `Epic-Id` + signed tag).
- `/home/willem/.claude/plugins/cache/loom-plugin/loom/0.1.0/skills/loom/references/schemas.md`
  §3.3 (assignment trailers — `Stack-Mode`, `Epic-Id` additions), §4.1
  (ASSIGNED required trailers — `Stack-Mode` joins the optional set, but
  required-on-root if opted in), §5.7 (orchestrator post-terminal commit —
  becomes the signed `stack-landed/<epic>` tag in stack-mode).
- `/home/willem/bitswell/bitswell/repos/bitswell/loom-tools/src/tools/pr-merge.ts`
  — the exact file that gains the stack-mode short-circuit (return
  `err('stack-mode-epic', ...)` when the epic opts in).
- `/home/willem/bitswell/bitswell/repos/bitswell/loom-tools/src/tools/pr-create.ts`
  — unchanged; `gh stack submit` creates the PRs for stack-mode epics, so
  `pr-create` is simply not called in that path.
- `/home/willem/bitswell/bitswell/repos/bitswell/loom-tools/src/tools/pr-retarget.ts`
  — unchanged and unused in stack-mode; retargeting is `gh stack`'s job.
- `/home/willem/bitswell/bitswell/repos/bitswell/loom-tools/src/tools/commit.ts`
  and `trailer-validate.ts` — the enforcement point for the two new
  trailers (`Stack-Mode`, `Epic-Id`).
- `/home/willem/bitswell/bitswell/repos/bitswell/loom-tools/src/tools/dag-check.ts`
  — the topological order input for `gh stack init --adopt`, and the fan-out
  detector that refuses stack-mode on non-linear DAGs.
- `/home/willem/bitswell/bitswell/repos/bitswell/loom-tools/src/tools/scope-check.ts`
  — gains a post-rebase re-check for stack-mode epics.
- GitHub issue #74 — the RFP; sharp edges #4 (merge vs rebase) and #5
  (worker-vs-orchestrator authority) are the two this angle bites
  hardest. Edge #4 is resolved by replacement, not reconciliation.
