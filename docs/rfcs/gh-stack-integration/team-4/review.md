# Team 4 — Review (glitch)

**Verdict:** APPROVED WITH EDITS — stays inside its angle, names the cost it's
paying, but glosses over four concrete failure modes that would sink the
implementation. Angle-fidelity is the strongest of the five teams. Execution
honesty is weaker than the prose suggests.

---

## 1. Coverage audit

All seven RFP sections are present and named by the correct headers:

| # | Required section           | Present | Header in proposal                         |
|---|----------------------------|---------|--------------------------------------------|
| 1 | Angle statement            | yes     | §1. Angle statement                        |
| 2 | Thesis                     | yes     | §2. Thesis                                 |
| 3 | What changes               | yes     | §3. What changes                           |
| 4 | Branch naming and scope    | yes     | §4. Branch naming and scope                |
| 5 | Worker authority           | yes     | §5. Worker authority — the core section    |
| 6 | End-to-end example         | yes     | §6. End-to-end example                     |
| 7 | Risks and rejected alts    | yes     | §7. Risks and rejected alternatives        |

Plus two appendices (A — referenced files, B — one-paragraph version). No
section is missing. No section is thin; §5 is the longest and most detailed,
which is appropriate given the outline named it the core.

---

## 2. Angle fidelity audit

**Verdict: the angle holds, unbroken, across every section.**

The outline's hard constraint was: "any sentence in this proposal implying
workers do not run `gh stack` commands is a bug." I scanned every section
looking for drift. I did not find any. Specific observations:

- **§1** repeats the angle three times in different registers (mechanical,
  architectural, reductive). The third register — *proximity of data beats
  protocol purity* — is the shortest accurate summary of the angle I've seen
  in any of the five proposals.
- **§2** uses the four-way rejected-alternatives framing to pressure-test
  its own angle by forcing every non-worker-driven path to explain how it
  handles mid-stack edits. The argument is not subtle, but it is honest.
- **§3** commits to file paths, role names, and tool names. No hand-waves
  about "a new tool would be added somewhere." The file paths are mostly
  right (one factual error; see §8 of this review).
- **§4** names the one-worker-per-stack constraint and justifies it from
  `gh-stack` internals (exit code 8, rerere locality, stack-file locking).
  This is the most angle-dependent architectural claim in the proposal,
  and it is argued from first principles rather than asserted.
- **§5** is the load-bearing section. It names *six* LOOM invariants being
  relaxed (workspace-write, agent-scope, `--no-ff` integration, the
  orchestrator-only tool precedent, worker-template §9, and the implicit
  one-agent-per-branch rule). Every one is quoted, cited, and justified.
  No other team's proposal enumerates its invariant relaxations this
  concretely.
- **§6** traces the auth → api → frontend flow end-to-end with actual
  `Stack-Op:` trailers, commit bodies, and a mid-stack edit followed by a
  conflict recovery. The worker is the `gh stack` caller on every line.
  There is no paragraph in which the orchestrator sneaks back into the
  call path.
- **§7** names rejected alternative A (post-hoc projection) and explicitly
  says it solves the audit-trail problem at the cost of the
  conflict-recovery problem. The inverse bet is stated directly.

There is **no paragraph that implies workers do not drive `gh stack`**. If
anything, the proposal overcommits to the angle — every failure mode is
answered by adding more worker authority rather than retreating to the
orchestrator. This is ideologically consistent, which is what the outline
asked for.

The only sentence that comes close to drift is in §6.9 Option A:

> promote each draft to ready (`gh pr ready`), merge in order via the
> merge queue

This is orchestrator-driven PR promotion *after* worker-driven stacking,
which is not drift — the angle is about the `gh stack` call path during
the work, and PR promotion at the end is allowed. But it creates a
correctness problem I'll address in §4 of this review (the double-apply
between local ff-merge and GitHub merge queue).

---

## 3. Sharp-edge audit

The RFP named five sharp edges. For each, I checked whether the proposal
(a) explicitly names it, and (b) gives a non-hand-wavy answer. Verdict
per edge:

### Sharp edge 1 — `loom-tools` already supports custom bases

**Proposal's answer:** not directly cited. The proposal talks about
`pr-retarget.ts` as "no changes because stack bases are set by `gh stack
submit`, not `pr-retarget`" (§3.1), but it does not engage with whether
the existing custom-base support in `loom-tools` could be used *instead*
of a new tool. This is a near-miss. A reviewer who believed custom-base
support was load-bearing for the epic would not be convinced by §3.1.

**Rating:** partially addressed; would benefit from one paragraph
acknowledging the custom-base feature and explaining why it does not
replace `stack-worker-init`.

### Sharp edge 2 — Dependency DAG exists

**Proposal's answer:** §3.1 and Appendix A reference `dag-check.ts`:
*"the topological sort runs over a single assignment whose Stack-Position
trailer declares the layer order, so the existing algorithm returns
[layer-1, layer-2, layer-3] unchanged."*

This is a crisp answer. It reuses the existing DAG machinery without
rewriting it, by treating stack-position as an intra-assignment ordering.
Good.

**Rating:** fully addressed.

### Sharp edge 3 — Branch naming conflict

**Proposal's answer:** §4.1 extends the convention to
`loom/<lead-agent>-<epic-slug>/<layer-slug>` and pins the `gh-stack`
prefix (`-p loom/<lead>-<epic>`) to make the `/` separator structural,
not accidental. §4.2 forces one worker per stack, which is how the
proposal avoids cross-agent collisions on the same namespace.

**Rating:** fully addressed for the single-worker case. **Not addressed
for the orchestrator-error case** — see chaos finding §4.A below.

### Sharp edge 4 — Merge vs rebase

**Proposal's answer:** §4 opens with the required direct statement
("This proposal breaks LOOM's `--no-ff` merge invariant for stack-epic
branches, and here is why the trade is worth it") and §5.2.3 spells
out cascading `--ff-only` as the replacement. The `--ff-only` → worker
commits become direct ancestors of `main` → trailer-based audit chain is
stated concretely in §5.4 with example `git log` output.

**Rating:** fully addressed at the argument level. **Not addressed at the
race-condition level** — see chaos finding §4.F below.

### Sharp edge 5 — Worker-vs-orchestrator PR authority

**Proposal's answer:** §5 is an entire section on this. The authority
boundary is drawn as *contracts vs mechanics*: orchestrator owns
assignments, scope, integration, and audit; worker owns commits,
branches, rebases, and draft submission. §5.6 enumerates what the
orchestrator retains. §5.8 enumerates four failure modes with
mitigations.

**Rating:** fully addressed at the angle level. This is the sharp edge
the proposal was *built* to answer, and it answers it directly in the
opposite direction from any other team. The "contracts vs mechanics"
frame is genuinely new.

**Sharp-edge summary:** 3 fully addressed, 2 partially. The two partials
(custom-base and branch-naming cross-agent case) are fixable by
paragraph-level additions, not structural changes.

---

## 4. Feasibility audit — chaos findings

This is the section where the proposal's gloss peels off. I tried to break
the proposal by imagining the worst adversary the proposal allows. Four
findings are material; two are paper cuts.

### 4.A. Two workers collide on the same stack namespace

**What the proposal says:** one worker per stack (§4.2), justified by
exit code 8, rerere locality, and stack-file locking. These are
*intra-worker* guarantees — they explain why two workers *cannot*
simultaneously operate on a shared worktree. They do **not** prevent two
*different* orchestrator-issued assignments from using overlapping
`loom/<lead-agent>-<epic-slug>/*` namespaces.

**The failure mode the proposal doesn't see:** the proposal assumes
namespace uniqueness is guaranteed by the orchestrator. It is not —
the orchestrator's only uniqueness guarantee today is the `loom/<agent>-<slug>`
prefix on the *primary* branch. There is no invariant preventing two
concurrent assignments with identical `<agent>-<slug>` values (e.g., a
retry after a failed assignment, or two orchestrator sessions racing on
a shared repo). If that happens, both workers `gh stack init -p
loom/ratchet-auth-stack auth-middleware` and then both force-push
`loom/ratchet-auth-stack/auth-middleware` with `--force-with-lease`. The
second push's lease is stale because the first push happened between
fetches, so it fails loudly. But the first worker's rebase state (which
includes the rerere cache for whatever conflicts it already resolved)
is now on disk in a worktree the second worker cannot read, and the
lease-failure error does not tell the second worker "you are a duplicate
assignment — abort."

**Worst case:** a retry-after-failure scenario. Worker A failed at 90%,
orchestrator issues worker B with the same `(agent, slug)`. Worker B
starts, finds `loom/ratchet-auth-stack/auth-middleware` already exists on
remote, assumes it's its own work from a prior compaction, fetches it,
and continues rebasing from worker A's last state. Worker A's state
includes commits worker B did not make. Worker B's subsequent `Stack-Op:`
trailers reference ops worker A performed. The audit trail is now
mislabeled: `Agent-Id: ratchet` on commits worker-B's session didn't
actually author.

**What the proposal needs:** a collision-detection step in the ASSIGNED
commit itself — orchestrator MUST verify no existing branches under the
target namespace before emitting `Stack-Epic: true`, and MUST fail
assignment (not silently overwrite) if any layer branch already exists.
Add this to §5.6 as retained orchestrator authority.

**Severity:** high. This is the single biggest hole.

### 4.B. Force-push outside scope is post-detected, not prevented

**What the proposal says:** §5.8.1 says a worker force-pushing outside
its stack namespace is "caught by general LOOM invariants (branch
protection on `main`, orchestrator-only integration for other agents'
branches, and `gh` auth scopes on the worker's token)" and "caught at
integration by mismatched reflog."

**The failure mode the proposal doesn't see:** none of those mitigations
are prevention. They are all detection, and detection happens *after*
the force-push has already overwritten history. Specifically:

1. **`gh auth` token scoping does not exist at the subcommand level.**
   The `stack-worker-init` tool wraps `gh stack` ops, but the tool cannot
   revoke the worker's shell access to raw `gh` or raw `git`. The
   "worker-stack-driver" role is a tool-dispatch check, not an OS-level
   capability. Any worker with shell access can run
   `GH_TOKEN=<...> gh api -X DELETE /repos/.../branches/main` and the
   tool cannot prevent it.
2. **Branch protection on `main` is not a LOOM invariant today.** The
   proposal says "must be caught by general LOOM invariants" but LOOM's
   protocol.md does not mandate branch-protection rulesets. Some repos
   have them, some don't.
3. **Integration-time reflog check cannot undo an overwrite.** If worker
   ratchet force-pushes `loom/moss-other-assignment/foo` to garbage at
   time T1, and the orchestrator integrates moss's assignment at T2, moss's
   work is already destroyed.

**What the proposal needs:** either (a) an explicit dependency on
branch-protection rulesets (and a pre-flight check that verifies they
are in place before `Stack-Epic: true` is issued), or (b) an
acknowledgement that the worker-stack-driver role is a trust tier, not
a sandbox, and name the operational mitigations (monitoring, rollback
procedures) that accompany trust grants.

**Severity:** medium — the attack surface is bounded to *peer* workers,
not `main`, in most configurations, but the proposal overstates the
containment.

### 4.C. `git_commits_since(branch, assigned_sha)` is uncomputable after rebase

**What the proposal says:** §4.4 pseudocode, step 2:

> ```
> commits = git_commits_since(branch, assigned_sha)
> ```

where `assigned_sha` is the ASSIGNED commit that lives on `loom/<lead>-<epic>`.

**The failure mode the proposal doesn't see:** in §5.2.3, the proposal
already admits: *"The original ASSIGNED commit is no longer an ancestor
of the worker's branches — it was rewritten out of history during the
first rebase."* After rebase, `assigned_sha` is **not reachable** from
any layer branch. Therefore `git log assigned_sha..layer-branch` is
uncomputable; git returns either the empty set (if the SHAs are
unrelated) or everything reachable from `layer-branch` (if you fall
back to `git log layer-branch`).

The integration scope-check algorithm in §4.4 depends on knowing which
commits the worker authored. If you can't walk from the ASSIGNED sha,
your alternatives are:

- `git log main..layer-branch` — this catches only commits since the
  orchestrator's last pull from `origin/main`, which may be stale or
  ahead.
- `git log --author=<agent>` — this trusts the commit header, which is
  a weaker trust root than a sha.
- `git log --grep='Agent-Id: <agent>'` — this trusts the trailer, which
  is what the worker is stamping, i.e. circular.

**What the proposal needs:** an explicit definition of the "fork point"
used by scope verification after rebase. The most defensible choice is
`merge-base(main, layer-branch)` *at the time the stack was submitted*,
which means the orchestrator must capture a fork-point sha at
`gh stack submit` time and verify against that. This is not expensive
but it is a real protocol change the proposal does not specify.

**Severity:** high. The scope-verification algorithm is the *key*
defence the proposal relies on, and it's uncomputable as written.

### 4.D. `Stack-Op:` trailers can be silently lost by rebase

**What the proposal says:** §5.4 — *"every rebase step is a
`Stack-Op: rebase` commit with its own timestamp."* §3.3 worker
template §10.3 — *"Every `gh stack` command you run must be followed
by a commit with a `Stack-Op: <op>` trailer. If the command produces
no file changes, use `git commit --allow-empty` to keep the audit
trail complete."*

**The failure mode the proposal doesn't see:** `git rebase` (which
powers `gh stack rebase`) drops empty commits by default unless
`--keep-empty` is passed. `gh stack rebase` does not document whether
it passes `--keep-empty` internally. The SKILL at line 628 documents
that `gh stack rebase` *"handles squash-merge detection and correctly
replays commits on top of the merge target"* — which means it is
actively rewriting commit identities during rebase. If the worker runs
`gh stack down` → `git commit --allow-empty -m '... Stack-Op: down'`
→ (some time passes) → `gh stack rebase --upstack`, the empty
`Stack-Op: down` commit may or may not survive the rebase. The
proposal *assumes* it survives. I believe it likely does not: `git
rebase` without `--keep-empty` drops empty commits, and `gh stack
rebase` is a `git rebase` under the hood.

**What the proposal needs:** either (a) `stack-worker-init` must set
`GIT_REBASE_OPTS=--keep-empty` or inject `--keep-empty` into every
rebase invocation, or (b) the audit design must stop relying on
`--allow-empty` commits and switch to a trailer format that persists
on real content commits only. The §6 end-to-end example uses empty
commits for navigation ops (`down`, `top`, `add`, `init`) — every one
of those is vulnerable to being dropped at the next rebase.

**Severity:** high. The audit contract is specifically the contract
the worker authority relaxation was supposed to preserve. A silent
drop of navigation-op trailers breaks that preservation claim.

### 4.E. `gh stack init` on an already-committed branch

**What the proposal says:** §6.2 — worker commits
`chore(loom): begin auth-stack` on `loom/ratchet-auth-stack`, then runs
`gh stack init -p loom/ratchet-auth-stack auth-middleware`.

**The failure mode the proposal doesn't see:** `gh stack init` creates
a *new* branch. After the command, the worker is on
`loom/ratchet-auth-stack/auth-middleware`. The ASSIGNED commit and the
"begin auth-stack" commit both live on `loom/ratchet-auth-stack`, which
is **not** one of the layer branches integration checks. `git merge
--ff-only loom/ratchet-auth-stack/auth-middleware` into `main` does
**not** bring the ASSIGNED commit along, because `auth-middleware` was
forked from `main` at `gh stack init` time, not from the commit that
carries `Stack-Epic: true`.

**Consequences:**

1. The ASSIGNED commit is never integrated. The audit trail on `main`
   has no `Task-Status: ASSIGNED` entry, no `Stack-Epic: true`
   declaration, no `Stack-Position:` trailer. The whole epic is
   invisible in `git log main`.
2. The `Scope:` trailer on the ASSIGNED commit is never in `main`'s
   history. Scope reconstruction after-the-fact requires walking the
   deleted `loom/ratchet-auth-stack` branch, which may not exist
   post-integration.
3. The state transition ASSIGNED → IMPLEMENTING → COMPLETED is visible
   only on the *non-integrated* branch. The integrated history shows
   only worker commits, no orchestrator assignment.

**What the proposal needs:** the orchestrator must either (a) cherry-pick
the ASSIGNED commit onto `main` as a separate "assignment record" commit
before the cascading ff merge, (b) use `gh stack init --adopt` on an
existing branch that already has the ASSIGNED commit, or (c) store the
assignment record in a LOOM-managed side branch. None of these are
specified in §4 or §6.9.

**Severity:** high. This breaks the audit contract that the entire
proposal leans on in §5.4.

### 4.F. Cascading `--ff-only` races with upstream `main` moves

**What the proposal says:** §6.9 —

> ```sh
> git checkout main
> git pull --ff-only
> git merge --ff-only loom/ratchet-auth-stack/auth-middleware
> git merge --ff-only loom/ratchet-auth-stack/api-endpoints
> git merge --ff-only loom/ratchet-auth-stack/frontend
> ```

**The failure mode the proposal doesn't see:** there is a time window
between the worker's `gh stack submit --auto --draft` (which force-pushes
all three layer branches after rebasing them onto `origin/main@T1`) and
the orchestrator's integration (at `T2 > T1`). If any other branch
merges to `main` in `[T1, T2]`, then `origin/main` has moved past the
point the layer branches were rebased onto. The first `git merge
--ff-only` fails loudly — layer-1 is not a fast-forward of the new main.

**Mitigation options, none specified by the proposal:**

1. Orchestrator re-runs `gh stack rebase` itself before integration.
   But the proposal forbids orchestrator `gh stack` calls — this
   contradicts the angle.
2. Orchestrator spawns the worker again to re-rebase. But worker
   dispatch is heavy, and re-dispatch after COMPLETED is not a LOOM
   state transition that currently exists.
3. Orchestrator falls back to Option B (close drafts, direct push to
   main). But direct push to main violates branch protection on most
   repos.
4. Orchestrator blocks all other integrations during a stack-epic
   integration window. But this is a global lock, which is exactly
   what the "proximity of data" argument said we should avoid.

**What the proposal needs:** a section on "what happens when `main`
moves between submit and integrate." This is the most likely operational
failure mode for any real-world use of the proposal.

**Severity:** medium-high. This is not a correctness flaw in the
angle, but a practical throughput ceiling the proposal pretends isn't
there.

### 4.G. `worker-stack-driver` as "superset of `worker`" — no such role exists

**What the proposal says:** §5.5 — *"a strict superset of `worker`"*.
§3.2 — *"a new role value `worker-stack-driver` is a strict superset of
`worker` for stack-epic assignments only"*. §3.1 — the `stack-worker-init`
tool is registered with `roles: ['worker-stack-driver']`.

**The factual error:** `repos/bitswell/loom-tools/src/types/role.ts`
defines the role enum as `'writer' | 'reviewer' | 'orchestrator'`.
There is no `worker` role. The `ProtocolRole` union is those three
values. The proposal either (a) is proposing to rename `writer` →
`worker` (not stated anywhere in §3) or (b) is proposing a role that
is a superset of a non-existent role.

Additionally, the proposal cites `src/types/tool.ts` as the file where
the role enum lives. The enum actually lives in `src/types/role.ts`;
`tool.ts` only imports `ProtocolRole` from `./role.js`. The tool
definition `roles` field is `readonly ProtocolRole[]`.

**What the proposal needs:** §3.2 must name the correct file
(`src/types/role.ts`) and the correct existing roles (`writer`,
`reviewer`, `orchestrator`). If the proposal wants to introduce a
stack-specific role, it is most naturally a superset of `writer`, not
`worker`. I've patched §3.2 to reflect this — see §8 of this review.

**Severity:** medium factual error; does not undermine the angle but
makes the implementation section wrong.

### 4.H. Minor paper cuts

- §4.2 says exit code 8 is the *reason* one worker owns the stack.
  SKILL line 785 says exit 8 is a 5-second timeout lock that callers
  should wait-and-retry. This is a lock-contention nuisance, not an
  architectural bar; the architectural bar is rerere locality and
  stack-file state, which the proposal also cites. The exit-8 citation
  overstates the mechanism.
- §5.8.4 claims `gh stack rebase --continue` is *"idempotent and
  recoverable from commit history."* It is recoverable from
  **on-disk** state (`.git/rebase-merge/`, `rerere` cache) only if a
  rebase is actually in progress. If the rebase already completed and
  the worker compacted before committing the `Stack-Op: rebase-continue`
  empty commit, `gh stack rebase --continue` errors with "no rebase in
  progress." The recovery procedure needs a step-0 check for rebase
  state.
- §6.5's mid-stack edit shows `Stack-Op: down` as an empty commit on
  `auth-middleware`. After `gh stack rebase --upstack`, that empty
  commit lives only on `auth-middleware`; it does not propagate to the
  upper layers. The §6.10 audit log implies all `Stack-Op:` trailers
  appear on `main` after integration, which is true *only if* the
  empty commits survive rebase (see chaos finding 4.D).
- §7.2 rejected-alternative D conflates "full `gh` grant" with "grant
  `gh stack` via tool." The middle ground — "grant `gh stack` via a
  raw-shell policy rather than a tool wrapper" — is not discussed.

### Feasibility summary

The proposal names six LOOM invariants to relax and honestly argues for
each relaxation. What it glosses over is the *mechanical plumbing* of
the replacement guarantees. Scope verification is uncomputable as
written (4.C). ASSIGNED commits vanish from history (4.E). Audit
trailers can be silently dropped by rebase (4.D). The namespace
collision case is unhandled (4.A). Three of those four are in the
category of *"the proposal assumes a guarantee git does not provide."*

All four are fixable at paragraph-level additions. None are structural
holes in the angle. The gap is between "the angle is coherent" and
"the implementation works" — the proposal is strong on the former and
quietly weak on the latter.

---

## 5. Strongest arguments

- **§5.4's trailer-based audit is strictly *more informative* than
  `--no-ff` merge structure.** This is the proposal's sharpest argument
  and it is correct: `--no-ff` records *landing events*, not
  *rebase events*, and rebases are where the interesting work happens.
  Every other team implicitly treats `--no-ff` as sacred. This
  proposal asks the right question: what is the audit trail *for*, and
  does the trailer format carry more signal than the structural
  format? The answer is yes, and the proposal is the only one to
  spell it out.
- **§2's four-way fight-the-grain argument is the tightest framing of
  the epic in any of the five proposals.** *"`gh-stack` was designed on
  the assumption that the person making the commits also drives the
  stack"* is a load-bearing sentence. It makes the other four
  approaches look like they are paying a tax to preserve an invariant
  (`--no-ff`) that may not be worth the tax.
- **§5.6's "contracts vs mechanics" distinction is a genuinely new
  trust-boundary frame.** Other teams either keep the orchestrator as
  gatekeeper or let workers run everything. This proposal carves a
  boundary that is *neither*: the orchestrator remains the verifier of
  *contracts* (scope, integration, audit) while the worker owns the
  *mechanics* (commits, branches, rebases). This is the kind of
  decomposition the epic was asking for.

---

## 6. Weakest arguments

- **The mechanical guarantees leak.** Scope verification depends on a
  fork point that no longer exists after rebase (4.C). The ASSIGNED
  commit is never integrated into `main`, so the very audit trail the
  proposal leans on is missing its first entry (4.E). `Stack-Op:`
  trailers on navigation ops are likely dropped by `git rebase`'s
  default empty-commit handling (4.D). Each of these is a detail the
  proposal glosses over with confident prose. A winner-selection
  reviewer who runs the audit-reconstruction query from §5.4 against a
  real stack epic will find at least one of these gaps on the first
  try, and the proposal's credibility depends on them *not* being
  there.
- **The "worker-stack-driver" role name is presented as a rename of
  an existing `worker` role that does not exist.** The LOOM
  `ProtocolRole` union is `writer | reviewer | orchestrator`. §3.2
  reads as if the proposal hasn't actually opened `role.ts`. This is
  the kind of error that makes a winner-selection reviewer stop
  trusting the rest of the implementation section. Fortunately it is
  a one-paragraph fix.
- **Namespace collision between retries/sessions is not addressed at
  all.** The single-worker-per-stack guarantee is argued from
  `gh-stack` internals but not from the orchestrator side; nothing
  prevents two concurrent assignments from using the same
  `loom/<agent>-<slug>/*` namespace, and the proposal's mitigation
  strategy ("integration-time detection") cannot undo a force-push
  overwrite that already happened.

The single thing most likely to sink this proposal in
winner-selection: **the trailer-based audit promise is not actually
delivered by the current spec.** The ASSIGNED commit vanishes, the
navigation-op trailers may be rebased away, and the scope verification
algorithm is uncomputable without a fork-point that the protocol does
not capture. If the winner-selection round tests any of these,
"APPROVED WITH EDITS" becomes "FAILED COMPLETENESS."

---

## 7. Should this win?

Not as-is. The angle is the strongest of the five. The execution is
the shakiest, because the other four teams keep the orchestrator in
the call path and therefore inherit LOOM's existing audit machinery
for free. This proposal is the *only* one that has to rebuild the
audit contract from scratch, which is both its most interesting claim
and its biggest risk.

**My recommendation:** this proposal should advance to winner-selection
contingent on a revision that addresses chaos findings 4.A (namespace
collision), 4.C (fork-point definition), 4.D (empty-commit rebase
drop), and 4.E (ASSIGNED commit integration). All four are paragraph-
level additions, not structural changes. The angle is worth the
revision.

If the revision is not made, the audit-trail argument in §5.4 collapses
and the proposal's central promise — that worker-driven stacking can
preserve an informative audit trail — becomes a claim that the current
text cannot back up.

---

## 8. Suggested edits to proposal.md

### Edits I made

1. **§3.2 factual correction — file path and existing role enum.**
   The role enum lives in `repos/bitswell/loom-tools/src/types/role.ts`,
   not `src/types/tool.ts`. `tool.ts` only imports `ProtocolRole` from
   `./role.js` and uses it as the type of the `roles: readonly
   ProtocolRole[]` field on `ToolDefinition`. The existing union is
   `'writer' | 'reviewer' | 'orchestrator'` — there is no `worker`
   role. I corrected §3.2 to name the right file and the right
   existing roles, and reframed `worker-stack-driver` as a strict
   superset of `writer` (which is what LOOM calls workers), not of a
   non-existent `worker` role. The substantive proposal — a new role
   tier gated on `Stack-Epic: true` — is preserved. Only the
   file-path and role-name references are corrected.

### Edits I did NOT make (author's voice preserved)

These are fixable issues I left for the author because they are
argumentative choices, not factual errors:

- **§4.2 overstates exit code 8** as the architectural bar for
  one-worker-per-stack. The real bar is `rerere` locality and
  stack-file cross-worktree unsafety; exit 8 is a 5-second lock
  timeout. This is a strength-of-argument issue, not a factual
  error — the author may have wanted the punchier citation.
- **§4.4 scope-verification pseudocode uses
  `git_commits_since(branch, assigned_sha)`** which is uncomputable
  after `gh stack rebase`. This needs an explicit fork-point
  definition; the author has to choose between `merge-base(main,
  layer-branch)@submit-time`, `git log --grep='Agent-Id:'`, or
  something else. I did not pick for them.
- **§5.8 failure modes do not include namespace collision between
  concurrent assignments** (chaos finding 4.A). The author should
  add a 5.8.5 addressing this.
- **§6.2's `gh stack init` call strands the ASSIGNED commit on a
  non-layer branch** (chaos finding 4.E). The integration phase in
  §6.9 needs either a cherry-pick step for the assignment record or
  a switch to `gh stack init --adopt`.
- **§5.4 and §10.3 rely on `--allow-empty` commits surviving
  rebase** (chaos finding 4.D). Either `stack-worker-init` must
  inject `--keep-empty` into rebase invocations, or the trailer
  format needs to move to real content commits.
- **§6.9 cascading `--ff-only` does not handle `main` moving between
  submit and integrate** (chaos finding 4.F). Needs a subsection.

These are all paragraph-level edits. None change the angle.

The author's voice, structural choices, and argument order are
preserved. I am a fact-checker and completeness auditor, not a
rewriter.

---

*— Reviewed by Glitch.*

*"If it doesn't break, you've confirmed it's real. If it does break,
you've found the drywall. This proposal has more real than drywall,
but the drywall is load-bearing."*
