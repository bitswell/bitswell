# Team 3 Review — Thorn

**Verdict:** APPROVED WITH EDITS

The proposal satisfies its angle contract (maximum protocol change via new
`Stack-*` trailers), covers all seven required sections, and is internally
coherent on the parts that do not touch the hardest case. The hardest case —
mid-stack rework reconciliation — is where the drywall is. The Verdict is
"approved with edits" rather than "request changes" because the core
trailer-extension thesis is defensible and implementable; but section 5
("Merge vs rebase") contains a load-bearing claim that does not survive
pressure, and the proposal should either fix it or name it explicitly as an
unresolved issue before this goes into the winner-selection round.

---

## 1. Coverage audit

All seven RFP-required sections are present, uniquely headed, and addressed
with specifics:

| Required section | Present? | Location |
|---|---|---|
| 1. Angle statement | yes | §1 |
| 2. What changes | yes | §3 (subsections 3.1–3.12) |
| 3. Branch naming and scope | yes | §4 |
| 4. Merge vs rebase | yes | §5 |
| 5. Worker authority | yes | §6 |
| 6. End-to-end example | yes | §7 (six phases, real trailer blocks) |
| 7. Risks and rejected alternatives | yes | §8 (six rejections, seven risks) |

No section is missing, no section is thin in the sense of being a
placeholder. §3 is exhaustive (12 subsections, insertion points, code
snippets) and §7 is the strongest — a concrete six-phase trace with
commit-level trailer blocks a reviewer can mentally run through
`trailer-validate`. The proposal also includes an unrequested §2 (Thesis)
and §9 (Summary), which are bonus, not filler.

No coverage gap to flag.

---

## 2. Angle fidelity audit

**Hard guardrail check:** team 3's angle is "first-class protocol primitive
with new `Stack-*` trailers." The proposal must not try to avoid schema
changes.

The proposal does not drift. §1 frames itself explicitly as "the maximum
protocol-change angle" and commits to the non-negotiable: stack-mode
ASSIGNED commits MUST carry `Stack-Id`, `Stack-Position`, `Stack-Base`, and
`Stack-Epic`. §3.1 opens by adding a new subsection 3.6 to `schemas.md` with
four new trailer entries. §3.3 adds seven new validation rules (15–21). §3.4
wires them into `trailer-validate.ts`. §3.5 extends `lifecycle-check.ts`,
§3.6 extends `dispatch.ts`, §3.7 extends `commit.ts`, §3.8 extends
`dag-check.ts`, §3.9 adds a new `mcagent-spec.md` MUST, §3.10 edits
`protocol.md` in two places. §3.11 changes the worker template. The diff is
large and unapologetic.

This is the right shape for an angle that is supposed to pay protocol debt
properly. No paragraph softens the schema change or tries to smuggle the
work into tooling-only territory. The proposal does not hedge on whether
schemas.md changes; it opens §3 by stating which section of schemas.md is
the "primary extension target" and specifies that rules 15–21 get appended.

The only angle-fidelity risk is §5.2's projection-layer design, which
*superficially* resembles Team 1's "publisher" angle. But team 3's frame is
crucially different: the projection is read from trailers, not from branch
name conventions. §5.2 explicitly says "the difference is that the topology
the projector publishes comes from trailers, not from filesystem conventions
or re-derived guesses." This is not drift — it is the correct separation of
concerns. `loom/*` is the protocol namespace (trailer-governed, never
rewritten); `stack/*` is the publication namespace (projection-owned,
freely rewritable). The trailer fact is the truth that drives the
publication.

**Angle fidelity verdict:** clean. Team 3 is unambiguously the
maximum-protocol-change team.

---

## 3. Sharp-edge audit

For each of the five RFP sharp edges:

### Sharp edge 1 — `loom-tools` already supports custom PR bases

**Named?** Obliquely, in the context of "publisher / projector" but not by
name. The proposal does not cite `pr-create` or `pr-retarget` and does not
say "the minimal-viable stacking path needs no tool changes at all." It
treats the existing custom-base support as out-of-scope because its
projection path goes through `gh stack init --adopt` and `gh stack submit
--auto --draft`, not through `pr-create`.

**Answer quality:** hand-waved. This is a live tension with the RFP's
reusability criterion ("Proposals that reuse existing LOOM primitives …
score higher than proposals that add parallel machinery"). Team 3 is
adding parallel machinery: a new `stack-project` recipe (§8.3), a new
cherry-pick-to-`stack/*` step, and a `gh stack` invocation path. The
`pr-create --base <branch>` primitive is never mentioned, even though it
would suffice for "create layer-2 PR against layer-1 branch" without the
`stack/*` namespace at all.

**Fix required:** §5.2 or §8.2 should explicitly rebut "why not just use
`pr-create --base loom/<downstack-agent>-<slug>`?" as a rejected
alternative. This is the most obvious alternative not listed in §8.2, and
its absence is conspicuous.

### Sharp edge 2 — Dependency DAG exists

**Named?** Yes, §8.2 rejected alternative #4 ("piggyback on `Dependencies`
alone") directly names `Dependencies` and explains why it is insufficient.
§3.3 rule 21 (`stack-dependencies-consistency`) makes the two consistent.
§3.8 extends `dag-check.ts` with `dag-stack-order-violation`.

**Answer quality:** strong. The proposal correctly observes that
`Dependencies` expresses a DAG with possible fan-out while `Stack-Base`
asserts linearity, cites SKILL.md limitation 1 ("stacks are strictly
linear") as the constraint that forces the stronger expression, and binds
the two together with a validator rule. The `dag-check.ts` extension is
correctly placed (after Kahn's algorithm, lines 142-165 — verified) and
emits a named violation rule the orchestrator can act on.

One small caveat: the proposal never says what happens to an epic that is a
DAG with fan-out. §1 and §8.2 both assume stack-mode is strictly linear, so
fan-out epics are simply not stack-mode — they fall back to today's
independent-PR or bundled-PR paths. This is coherent but narrows the
thesis: the protocol primitive being added is only usable by a subset of
epics (single-parent single-child chains). The proposal does not say this
out loud.

### Sharp edge 3 — Branch naming conflict

**Named?** Yes, §4.1 opens with it. §4.1 also lists the four specific
things that break if you overload branch names with topology (assignment
slug mapping in schemas.md §2, `dispatch.ts` branch-verify, validators that
parse branch names, `@<agent>` mention UX).

**Answer quality:** strong. §4 commits to `loom/<agent>-<slug>` and argues
that stack position is a trailer, not a branch-name fact. §4.4 names the
new failure mode (two ASSIGNED branches sharing `Stack-Id`+`Stack-Position`)
and explains how it is caught at dispatch time with a named rule. The
split-namespace design in §5 (loom/* for protocol, stack/* for
publication) cleanly resolves who owns which prefix.

The only loose end: the `stack/<epic-slug>/<NN>-<slug>` naming scheme is
introduced in §5.2 without any discussion of conflicts across epics — e.g.,
two epics that pick the same slug, or an epic whose slug collides with a
`gh-stack` reserved name. Not a dealbreaker, but a missed opportunity to be
exhaustive.

### Sharp edge 4 — Merge vs rebase

**Named?** Yes, §5 is the entire answer, and it is the longest subsection.

**Answer quality:** mixed. The structural answer (two disjoint namespaces,
`loom/*` never rebased, `stack/*` disposable and freely rebaseable) is
sound. The thesis that `gh-stack`'s rebase model and LOOM's `--no-ff` audit
model can coexist if they operate on different namespaces is genuinely new
relative to the other bolt-on approaches, and it is the one place this
proposal earns its complexity budget.

The rebase-reconciliation story in §5.3–§5.4, however, does not hold up.
This is the load-bearing claim I pressure-tested. See **section 4
(feasibility audit) below** for the full breakdown. Summary: the
`stack-base-stale` lifecycle rule as defined cannot fire in the scenario
the proposal wants it to catch, because LOOM never mutates `loom/*`
branches — so the "current HEAD of `Stack-Base` branch" never changes, and
there is never a staleness to detect. The staleness is hidden in a
different place (which COMPLETED branch the projector picks), and the
detector is looking in the wrong place.

### Sharp edge 5 — Worker vs orchestrator PR authority

**Named?** Yes, §6.4 is titled "Workers never invoke `gh stack`" and is
absolute: "Workers do not run `gh stack init`, `gh stack add`, `gh stack
submit`, `gh stack rebase`, `gh stack sync`, or any other `gh stack`
command. The `gh-stack` SKILL.md is not loaded into worker sessions."

**Answer quality:** strong. The invariant matrix in §6.5 is explicit: every
existing "orchestrator-only" row is preserved verbatim. Three new rows are
added ("stack identity is orchestrator-owned", "stack-base-ready dispatch
gate", "stack-mode workers read `STACK_BASE_BRANCH`") and each is
characterised as strictly additive or strictly stronger than an existing
invariant, not a relaxation.

One quibble: §4.3 grants stack-mode workers a new read privilege ("MAY
read the tree of its `Stack-Base` branch via `git show`"). The proposal
correctly notes this was already legal today via cross-worktree git reads
in multi-worktree setups, so the proposal is documenting an accident, not
granting new power. But the claim "there is no way for a stack-mode worker
to reach through the `Stack-Base` pointer into the filesystem and write"
in §6.3 depends on `validateScope` in `commit.ts` running correctly, which
it already does. OK.

**Sharp-edge summary:** 3 strong (edges 2, 3, 5), 1 hand-waved (edge 1 —
custom PR base), 1 broken (edge 4 — the rebase reconciliation detector).

---

## 4. Feasibility audit

Let me size the work the proposal claims and flag the two underestimates.

### 4a. What's sized honestly

The diff is bounded and the files touched are named correctly. I verified:

- `trailer-validate.ts` is 310 lines, ASSIGNED block is lines 214–247
  (proposal says 213–247 — close enough), `Scope-Expand` format checks are
  lines 279–305 (exact), `TASK_STATUS_ENUM` is lines 34–41 (proposal says
  line 34 — exact), `HEARTBEAT_RE` is line 51 (exact).
- `lifecycle-check.ts` is 368 lines, `TERMINAL_STATES` is lines 36–39
  (exact), `LEGAL` state table is lines 47–60 (exact), post-walk invariants
  are lines 337–363 (exact).
- `dispatch.ts` is 81 lines, branch verify is 42–53 (exact), worktree
  creation is 55–72 (exact), return is 74–79 (exact — proposal says "line
  55" for insertion point, correct).
- `commit.ts` is 126 lines, auto-inject block is lines 66–72 (exact),
  `validateScope` is at line 43 (exact), `ctx.branch` is available on the
  context (verified at lines 47, 116).
- `dag-check.ts` is 202 lines, `AgentEntry` is lines 5–10 (exact), Kahn's
  algorithm is lines 142–165 (exact).

Every file path and line range in §3 checks out. This is unusually good
sourcing for an RFP proposal. The writer went to the files.

The seven-new-rules / ~21-test estimate in §8.1 is realistic for the
`trailer-validate`, `commit`, and `dag-check` edits. Each rule is a ~20-line
addition to an existing pattern. The auto-injection in `commit.ts` is a
single `readBranchStackContext` helper plus a three-line spread into
`allTrailers`.

### 4b. What's underestimated

**4b.i — The `readBranchStackContext` helper in `commit.ts` assumes the
"most recent Task-Status: ASSIGNED commit" is the worker's own, but does
not grapple with worktree ancestry.**

§3.7's proposed helper runs `git log --grep=Task-Status: ASSIGNED -1
--format=%(trailers) <branch>`. This picks the most recent commit matching
the grep, walking from `<branch>`'s HEAD backwards in time. In a stack
scenario, moss's `loom/moss-api-endpoints` branch is *created* by the
orchestrator. The proposal never says what it branches FROM.

- If it branches from `main`, moss's ASSIGNED commit is the only ASSIGNED
  commit reachable from its HEAD, and the grep-1 is correct.
- If it branches from `loom/ratchet-auth-middleware` (the natural choice if
  moss wants to actually compile against layer 1 without manual setup),
  then moss's branch has TWO `Task-Status: ASSIGNED` commits in its
  ancestry — ratchet's and moss's. `git log --grep -1` returns the most
  recent by commit time, which is moss's if the orchestrator writes
  assignments in order. But this depends on commit ordering, not on
  branch topology.

The proposal needs to say which base `dispatch.ts` creates the worker's
worktree from, and whether `readBranchStackContext` uses `base..branch`
(which bounds the walk correctly) or just `branch` (which does not). §3.7's
current snippet uses `branch`, which is fragile.

**4b.ii — §5.3 `Stack-Base-SHA` and §3.5 `stack-base-stale` are
inconsistent with each other and with LOOM's immutability invariant.**

This is the drywall. Pressure test:

- §3.1 defines `Stack-Base-SHA` as "the integrator-observed SHA of the
  downstack neighbor (or trunk) at the moment this branch was integrated."
- §7 phase 4 writes `Stack-Base-SHA: 1234…ef` on moss's integration commit
  and parenthesizes "which is `loom/ratchet-auth-middleware`'s HEAD at the
  moment of integration — which equals the merge commit created in phase 2."
  This parenthetical is **incoherent**. `loom/ratchet-auth-middleware` is a
  worker branch; the merge commit from phase 2 is on `main`, not on
  `loom/ratchet-auth-middleware`. A `--no-ff` merge of
  `loom/ratchet-auth-middleware` into `main` does not move the tip of
  `loom/ratchet-auth-middleware`. The tip of the worker branch remains the
  worker's COMPLETED commit. So the "which equals" gloss is wrong.
- §3.5's `stack-base-stale` rule walks the integration log, "resolves
  `Stack-Base` to a branch, reads its current HEAD SHA, compares to this
  branch's `Stack-Base-SHA` trailer." `Stack-Base: ratchet/auth-middleware`
  resolves to `loom/ratchet-auth-middleware`, whose HEAD is its (single,
  immutable) COMPLETED commit. Because LOOM's invariant (§6.1 of this
  proposal, preserved verbatim) is that `loom/*` branches are never
  rewritten, that HEAD **never moves**.
- §5.4 and §7 phase 5 acknowledge this implicitly: the rework pattern
  creates a *new* branch, `loom/moss-auth-middleware-fix`, with the same
  `Stack-Id` and the same `Stack-Position: 1`, not a new commit on
  `loom/ratchet-auth-middleware`. The old branch is left in history
  untouched.
- Therefore: `loom/ratchet-auth-middleware`'s HEAD after phase 5 is still
  the same SHA as before phase 5. `stack-base-stale` compares
  `Stack-Base-SHA` on moss's integration commit (captured in phase 2) to
  the current HEAD of `loom/ratchet-auth-middleware` (unchanged). They are
  **always equal**. The rule cannot fire.

The staleness the proposal is actually trying to detect is "there is a
newer COMPLETED branch for the same `(Stack-Id, Stack-Position)` than the
one moss was built against." That is a projection-layer question, not a
branch-HEAD question. It requires the detector to enumerate all ASSIGNED
branches for a given `(Stack-Id, Stack-Position)`, order them by COMPLETED
integration time, and flag moss if there is a newer one. That is what the
projector in §5.2 does implicitly when it "picks the latest COMPLETED
branch per position." But `lifecycle-check`'s `stack-base-stale` rule, as
specified, is looking at the wrong thing.

The fix is small but the writer must acknowledge it: either redefine
`Stack-Base-SHA` to be "SHA of the merge commit on trunk that introduced
the downstack branch's content" (and then the detector walks the integrated
merge log on trunk looking for a *newer* merge at the downstack position),
or redefine `stack-base-stale` to compare the set of COMPLETED branches per
`(Stack-Id, Stack-Position)` rather than a single branch HEAD.

The core idea — that staleness is detectable from trailers — is
salvageable. The specific mechanism in §3.5 and §5.3 as written is not.
This is the one thing that could sink this proposal in winner selection:
the review criteria explicitly reward "honest about tradeoffs" over
hand-waves, and the current text reads as confident where it should read
as tentative.

**4b.iii — The `trailer-validate` vs multi-commit rule split is cleanly
stated but the `stack-position-unique` enforcement in §3.6 is quietly
global.**

§3.4 correctly pushes multi-commit invariants (`stack-position-unique`,
`stack-base-resolvable`, `stack-base-position`, `stack-dependencies-
consistency`) into `lifecycle-check` and `dispatch`. But §3.6's uniqueness
check runs a `git log --all --grep='Stack-Id: <id>'` across all branches
with a matching `Stack-Id`. The proposal calls this "O(N) in the number of
unintegrated ASSIGNED commits for one `Stack-Id`, bounded by stack depth
(<10)."

This is fine for the one-stack case, but it is the first cross-branch
check in `dispatch.ts`. Every other dispatch check is scoped to the branch
being dispatched. The new check adds a dependency on `--all`, which crosses
the worktree boundary (refs from sibling worktrees are visible, but only if
dispatch is run from the main repo, not from inside a worker's worktree).
The proposal needs to specify **where** dispatch runs — the main checkout
or a worker worktree — and confirm the `--all` scope is the right one.

Also, `git log --all --grep='A' --grep='B' --all-match` has subtle
behaviour: `--all-match` requires BOTH greps to match in the same commit's
*message*, which is correct for matching "Task-Status: ASSIGNED" AND
"Stack-Id: <id>" in a single commit. But the `%H %(trailers:key=Stack-
Position,valueonly)` format string in the snippet is not quoted
consistently and will produce a multi-line output per commit (trailers
contain newlines). The parser in the snippet is handwritten as a comment
("// parse lines, count positions equal to stack.stackPosition") with no
detail. The implementer will have to write that parser correctly, and it
is less trivial than the comment suggests. Flag: ~40 lines of real code
behind the "parse lines" comment, not 5.

**4b.iv — `mcagent-spec.md` R.19 conformance depends on runtime adapters
the proposal doesn't control.**

§3.9 adds R.19: "A conforming agent runtime MUST … set the corresponding
`STACK_ID`, `STACK_POSITION`, `STACK_BASE_BRANCH`, and `STACK_EPIC`
environment variables on the worker process." This is a runtime contract
change. §8.1's "runtime-adapter conformance" risk names the issue and
mitigates it with a new test suite — "outside this proposal's scope but
signalled in the follow-up." That is a real deferred cost. The proposal
does not size the conformance-test effort and does not say which runtimes
exist today that would need to implement it (claude-code? others?). For a
"big PR, pay the debt now" angle, this is a surprising corner to defer.

### 4c. Feasibility summary

The seven new validation rules and the `trailer-validate.ts`/`dag-check.ts`
edits are honestly sized and implementable in a week. The `commit.ts`
auto-injection is a 20-line extension. The `dispatch.ts` gate is ~40 lines
of honest code plus the parsing work the proposal elides. The
`schemas.md` / `protocol.md` / `mcagent-spec.md` doc edits are mechanical.

The rebase-reconciliation (§5.3–§5.4) is the one thing the proposal cannot
implement as described. It must be rewritten — either the `Stack-Base-SHA`
pivot is redefined to reference an integration-merge SHA on trunk, or the
`stack-base-stale` rule is redefined to operate on the set of COMPLETED
branches per `(Stack-Id, Stack-Position)`. Until then, the proposal
overclaims on rework detection.

---

## 5. Strongest arguments

The strongest argument is the **immutability-preserving split namespace**
in §5: `loom/*` for protocol (trailer-governed, never rewritten) and
`stack/*` for publication (projection-owned, freely rebaseable). This is
the only proposal angle that can simultaneously satisfy "LOOM never
rebases" AND "gh-stack rebases and force-pushes" without relaxing either.
The trailer-as-source-of-truth framing means the projection is always
reproducible from the audit trail, which is exactly the property LOOM
cares about.

The **line-level sourcing in §3** is a second strong argument. Every file
path, every line range, every insertion point I verified is correct. This
is unusually good fidelity for a design proposal and makes the
implementation diff easy to reason about — which is exactly the
"implementable by another team without re-doing the design" bar the RFP
asked for.

The **invariant matrix in §6.5** is a third. It is the cleanest
articulation in any RFP proposal of what LOOM's existing invariants are,
which are preserved, and which are strictly added — and it makes
verification straightforward: every row that was "yes" before is still
"yes," and three new rows appear as "yes (new)." Nothing moves from "yes"
to "relaxed."

---

## 6. Weakest arguments

The one thing that could sink this proposal in winner selection is §5.3
and the `stack-base-stale` detector. The entire rework-detection story
depends on a trailer that, as defined, cannot detect the thing it is
designed to detect, because LOOM's immutability invariant guarantees the
compared-against SHA never moves. A reviewer in the winner round who
traces phase 5 carefully will conclude the mechanism is incoherent and
mark the proposal down accordingly. The fix is small but it must happen
before winner selection, not after.

A secondary weakness is the **absence of `pr-create --base` as a rejected
alternative in §8.2**. The RFP explicitly names "`loom-tools` already
supports custom PR bases" as sharp edge 1, and the most obvious
minimum-viable rival to this proposal is "use `pr-create --base` to target
the downstack branch directly and skip `gh stack` entirely." The proposal
does not name this alternative or explain why it does not suffice. The
omission will be noticed.

A tertiary weakness is the **quiet narrowing of the thesis to
strictly-linear epics**. §8.2 rejected alternative #4 correctly observes
that stacks are strictly linear, but the proposal never says "fan-out
epics are not stack-mode and fall back to today's workflow." A reviewer
who asks "what about the DAG epic with three parallel subtasks feeding
into one integration?" finds no answer. The machinery being added only
applies to single-parent single-child chains, which is a smaller fraction
of LOOM epics than §2 implies.

---

## 7. Sharp-edge-on-the-writer

One adversarial question the writer should have answered in §8.1 risks but
did not: **what does `commit.ts`'s auto-inject do if the orchestrator
explicitly passes `Stack-Id: <different-uuid>` in `input.trailers`?**

§3.7's snippet uses `...(input.trailers ?? {})` as the final spread, so
caller-supplied trailers win. §3.7 notes this explicitly: "Caller-supplied
trailers take precedence, so the orchestrator can still explicitly set
`Stack-Id` on a fresh ASSIGNED commit without a read-back loop." Good —
but this opens a failure mode. A worker could, today, call the commit
tool with `trailers: {'Stack-Id': 'deadbeef-…'}` and override the branch's
actual `Stack-Id`. The `commit.ts` tool has `roles: ['writer',
'orchestrator']` at line 34 of the existing file, so workers can call it
directly. Rule 20 `stack-trailers-inheritance` in `lifecycle-check`
catches this post-hoc, but only at branch-validation time, not at commit
time. `trailer-validate` could catch it per-commit if given access to the
branch's prior ASSIGNED context, but §3.4 explicitly says per-commit
validation does not do multi-commit reasoning.

The proposal should either (a) strip `Stack-Id` and `Stack-Position` from
`input.trailers` for non-orchestrator callers in `commit.ts`, or (b)
add a per-commit check in `trailer-validate` that compares the
commit's `Stack-Id` against the branch's ASSIGNED-commit `Stack-Id` (which
crosses the "per-commit only" boundary and is a real architectural
change), or (c) acknowledge the late-binding check and accept that a
worker can write a bad commit that is rejected at the next
`lifecycle-check` sweep. None of these are spelled out.

---

## 8. Suggested edits to proposal.md

I made **zero** edits to `proposal.md`. Every file path and line reference
I spot-checked in §3 is correct. No factual errors of the kind reviewers
are authorised to correct (wrong file, wrong line number, wrong tool name)
were found. The issues I raised are substantive design issues, not
fact-check fixes, and the review guardrails say the writer's voice is
preserved on design matters.

If the writer wants to fix the review-round weaknesses before winner
selection, the minimal edit set would be:

1. **Fix §5.3 `Stack-Base-SHA` semantics and §3.5 `stack-base-stale`
   rule.** Either redefine `Stack-Base-SHA` as the merge-commit-on-trunk
   SHA (so the detector compares against the current merge-commit-on-trunk
   for the downstack position, which *does* change on rework), or redefine
   `stack-base-stale` to walk all COMPLETED branches for a given
   `(Stack-Id, Stack-Position)` and compare integration timestamps. Also
   fix §7 phase 4's parenthetical gloss ("which equals the merge commit
   created in phase 2") — this is factually wrong about where the merge
   commit lives.
2. **Add a rejected alternative #7 in §8.2: "use `pr-create --base
   loom/<downstack-agent>-<slug>` and skip the `stack/*` namespace
   entirely."** Explain why the projection-through-`stack/*` path is worth
   the added machinery given that the RFP's sharp edge 1 specifically
   names custom PR bases as the minimum viable path.
3. **Add one paragraph to §1 or §2.4 explicitly naming the narrowed scope:
   "Stack-mode is for strictly-linear epics; fan-out DAG epics are
   unaffected by this proposal and continue to use today's
   independent-PR / bundled-PR paths."**
4. **Add one paragraph to §3.7 or §6.2 addressing the worker-override
   vector on `Stack-Id` / `Stack-Position` in `commit.ts`'s
   caller-supplied trailer precedence.**
5. **Specify in §3.6 from which `cwd` dispatch runs the new
   `readStackContext` helper, so the `--all` scope of
   `stack-position-unique` is unambiguous.**

None of these is a style fix. All five are substantive clarifications
that would move the proposal from "approved with edits" to unambiguously
approved.

---

**Thorn strike:** I pressure-tested the `Stack-Base-SHA` / `stack-base-
stale` mechanism and it cannot fire under LOOM's own immutability
invariant — the detector's comparand is a branch HEAD that, by §6.1, never
moves. The rework-detection story in §5 is drywall, not a load-bearing
wall. Everything else holds.

— Reviewed by Thorn
