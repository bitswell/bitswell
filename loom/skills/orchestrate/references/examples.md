# LOOM Examples

**Protocol**: `loom/2`

Concrete usage examples for the LOOM orchestrate workflow.

---

## 1. Assign and Dispatch an Agent

```bash
# 1. Create branch
git checkout main
git checkout -b loom/ratchet-plugin-scaffold

# 2. Commit the assignment
git commit --allow-empty -m "$(cat <<'EOF'
task(ratchet): scaffold loom plugin directory structure

Create the initial loom/ plugin directory:
- loom/bin/ for CLI commands
- loom/skills/ for skill definitions
- loom/agents/ for worker agent definitions

Acceptance criteria:
- loom/bin/loom-dispatch and loom/bin/loom-spawn exist and are executable
- loom/skills/orchestrate/SKILL.md exists
- loom/agents/loom-worker.md exists

Agent-Id: bitswell
Session-Id: f8309ae8-ac76-4766-8926-362cdd06d04b
Task-Status: ASSIGNED
Assigned-To: ratchet
Assignment: plugin-scaffold
Scope: loom/**
Dependencies: none
Budget: 100000
EOF
)"

# 3. Dispatch via loom-dispatch
loom-dispatch --branch loom/ratchet-plugin-scaffold
```

---

## 2. Check Agent Status

```bash
# Is it done?
git log -1 --format='%(trailers:key=Task-Status,valueonly)' \
  --grep='Task-Status:' loom/ratchet-plugin-scaffold
# → COMPLETED

# What did it find?
git log --format='%(trailers:key=Key-Finding,valueonly)' \
  loom/ratchet-plugin-scaffold | grep -v '^$'
# → loom/bin/ created with two executable scripts
# → SKILL.md follows Claude Code skill format

# Is the agent still alive? (liveness)
git log -1 --format='%(trailers:key=Heartbeat,valueonly)' \
  loom/ratchet-plugin-scaffold
# → 2026-04-03T14:45:00Z
```

---

## 3. Chained Dependencies

```bash
# Assignment 1: scaffold plugin
git checkout -b loom/ratchet-plugin-scaffold
git commit --allow-empty -m "$(cat <<'EOF'
task(ratchet): scaffold loom plugin

...

Agent-Id: bitswell
Session-Id: abc123
Task-Status: ASSIGNED
Assigned-To: ratchet
Assignment: plugin-scaffold
Scope: loom/**
Dependencies: none
Budget: 100000
EOF
)"

# Assignment 2: write worker agent (depends on scaffold)
git checkout main
git checkout -b loom/moss-loom-worker
git commit --allow-empty -m "$(cat <<'EOF'
task(moss): write loom-worker agent definition

Create loom/agents/loom-worker.md — the worker spawned by loom-dispatch.

...

Agent-Id: bitswell
Session-Id: abc123
Task-Status: ASSIGNED
Assigned-To: moss
Assignment: loom-worker
Scope: loom/agents/**
Dependencies: ratchet/plugin-scaffold
Budget: 80000
EOF
)"

# Scan — loom-dispatch will dispatch ratchet immediately,
# hold moss until ratchet/plugin-scaffold is COMPLETED
loom-dispatch --scan
```

---

## 4. Integrate Completed Work

```bash
# Verify completed
git log -1 --format='%(trailers:key=Task-Status,valueonly)' \
  --grep='Task-Status:' loom/ratchet-plugin-scaffold
# → COMPLETED

# Merge into workspace, pausing before the merge commit
git checkout main
git merge --no-ff --no-commit loom/ratchet-plugin-scaffold

# Commit the integration with orchestrator trailers
git commit -m "$(cat <<'EOF'
chore(loom): integrate ratchet/plugin-scaffold

Agent-Id: bitswell
Session-Id: f8309ae8-ac76-4766-8926-362cdd06d04b
EOF
)"
```

---

## 5. Retry a Resource-Limited Agent

An agent exits BLOCKED due to token budget exhaustion. The orchestrator creates a new branch with a higher budget.

```bash
# Check state — agent exited BLOCKED with resource_limit
git log -1 --format='%(trailers:key=Task-Status,valueonly)' \
  --grep='Task-Status:' loom/ratchet-plugin-scaffold
# → BLOCKED

git log -1 --format='%(trailers:key=Blocked-Reason,valueonly)' \
  loom/ratchet-plugin-scaffold
# → resource_limit

# Create a new branch for the retry (do NOT reuse the blocked branch)
git checkout main
git checkout -b loom/ratchet-plugin-scaffold-retry

git commit --allow-empty -m "$(cat <<'EOF'
task(ratchet): scaffold loom plugin (retry -- resource_limit)

Same task as plugin-scaffold. Previous attempt exited BLOCKED due to
token budget. Increased budget to 200000.

...

Agent-Id: bitswell
Session-Id: f8309ae8-ac76-4766-8926-362cdd06d04b
Task-Status: ASSIGNED
Assigned-To: ratchet
Assignment: plugin-scaffold-retry
Scope: loom/**
Dependencies: none
Budget: 200000
EOF
)"

loom-dispatch --branch loom/ratchet-plugin-scaffold-retry
```

---

## 6. Scan for All Pending Work

```bash
# Show status of all loom/* branches
for b in $(git branch --list 'loom/*' --format='%(refname:short)'); do
  s=$(git log -1 --format='%(trailers:key=Task-Status,valueonly)' \
    --grep='Task-Status:' "$b" 2>/dev/null | head -1 | xargs)
  printf "%-50s %s\n" "$b" "${s:-unknown}"
done
```

```
loom/moss-loom-worker                              ASSIGNED
loom/ratchet-orchestrator-skill                    COMPLETED
loom/ratchet-plugin-scaffold                       IMPLEMENTING
loom/vesper-reconcile-docs                         COMPLETED
```

---

## 7. Heartbeat Monitoring

```bash
# Check all running agents for stale heartbeats (>5 min old)
now=$(date -u +%s)
for b in $(git branch --list 'loom/*' --format='%(refname:short)'); do
  s=$(git log -1 --format='%(trailers:key=Task-Status,valueonly)' \
    --grep='Task-Status:' "$b" 2>/dev/null | head -1 | xargs)
  [[ "$s" != "IMPLEMENTING" ]] && continue
  hb=$(git log -1 --format='%(trailers:key=Heartbeat,valueonly)' "$b" | head -1 | xargs)
  [[ -z "$hb" ]] && echo "STALE (no heartbeat): $b" && continue
  hb_epoch=$(date -u -d "$hb" +%s 2>/dev/null || date -u -j -f "%Y-%m-%dT%H:%M:%SZ" "$hb" +%s 2>/dev/null)
  age=$(( now - hb_epoch ))
  [[ $age -gt 300 ]] && echo "STALE ($age s): $b" || echo "OK ($age s): $b"
done
```

---

*End of LOOM Examples.*
