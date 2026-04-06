//! State machine model and transition validator for the LOOM protocol.
//!
//! Based on:
//! - `loom/skills/orchestrate/references/protocol.md` §2 (state machine)
//! - `loom/skills/orchestrate/references/schemas.md` §4, §7 (guards, validation rules)

use std::collections::HashMap;

// ── State ────────────────────────────────────────────────────────────────────

/// All possible task states in the LOOM lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Assigned,
    Implementing,
    Completed,
    Blocked,
    Failed,
}

impl TaskStatus {
    /// Returns true for states with no valid outgoing transitions.
    pub fn is_terminal(self) -> bool {
        matches!(self, TaskStatus::Completed | TaskStatus::Failed)
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.trim() {
            "ASSIGNED" => Some(TaskStatus::Assigned),
            "IMPLEMENTING" => Some(TaskStatus::Implementing),
            "COMPLETED" => Some(TaskStatus::Completed),
            "BLOCKED" => Some(TaskStatus::Blocked),
            "FAILED" => Some(TaskStatus::Failed),
            _ => None,
        }
    }
}

// ── Commit trailers ───────────────────────────────────────────────────────────

/// Minimal parsed commit trailers for state machine validation.
#[derive(Debug, Clone, Default)]
pub struct CommitTrailers {
    pub task_status: Option<TaskStatus>,
    pub agent_id: Option<String>,
    pub session_id: Option<String>,
    pub heartbeat: Option<String>,
    // COMPLETED trailers
    pub files_changed: Option<i64>,
    pub key_findings: Vec<String>,
    // BLOCKED / FAILED trailers
    pub blocked_reason: Option<String>,
    pub error_category: Option<String>,
    pub error_retryable: Option<String>,
    // ASSIGNED trailers
    pub assigned_to: Option<String>,
    pub assignment: Option<String>,
    pub scope: Option<String>,
    pub dependencies: Option<String>,
    pub budget: Option<i64>,
}

impl CommitTrailers {
    /// Parse trailers from a commit message string.
    ///
    /// Recognises `Key: Value` lines anywhere in the message; multi-value
    /// trailers (e.g. `Key-Finding`) accumulate into a `Vec`.
    pub fn parse(message: &str) -> Self {
        let mut map: HashMap<String, Vec<String>> = HashMap::new();

        for line in message.lines() {
            let Some(colon) = line.find(": ") else { continue };
            let key = line[..colon].trim();
            let val = line[colon + 2..].trim();
            // Trailer keys are kebab-case alphanumeric — skip prose lines.
            if key.is_empty()
                || val.is_empty()
                || !key.chars().all(|c| c.is_alphanumeric() || c == '-')
            {
                continue;
            }
            map.entry(key.to_string()).or_default().push(val.to_string());
        }

        let get = |k: &str| map.get(k).and_then(|v| v.first()).cloned();
        let get_all = |k: &str| map.get(k).cloned().unwrap_or_default();

        CommitTrailers {
            task_status: get("Task-Status").and_then(|s| TaskStatus::from_str(&s)),
            agent_id: get("Agent-Id"),
            session_id: get("Session-Id"),
            heartbeat: get("Heartbeat"),
            files_changed: get("Files-Changed").and_then(|s| s.parse().ok()),
            key_findings: get_all("Key-Finding"),
            blocked_reason: get("Blocked-Reason"),
            error_category: get("Error-Category"),
            error_retryable: get("Error-Retryable"),
            assigned_to: get("Assigned-To"),
            assignment: get("Assignment"),
            scope: get("Scope"),
            dependencies: get("Dependencies"),
            budget: get("Budget").and_then(|s| s.parse().ok()),
        }
    }
}

// ── Transition table ──────────────────────────────────────────────────────────

/// Returns true iff the transition `from` → `to` is legal per the protocol.
///
/// Legal transitions (schemas.md §7.3):
/// - ASSIGNED → IMPLEMENTING
/// - IMPLEMENTING → COMPLETED | BLOCKED | FAILED
/// - BLOCKED → IMPLEMENTING | FAILED
pub fn is_valid_transition(from: TaskStatus, to: TaskStatus) -> bool {
    matches!(
        (from, to),
        (TaskStatus::Assigned, TaskStatus::Implementing)
            | (TaskStatus::Implementing, TaskStatus::Completed)
            | (TaskStatus::Implementing, TaskStatus::Blocked)
            | (TaskStatus::Implementing, TaskStatus::Failed)
            | (TaskStatus::Blocked, TaskStatus::Implementing)
            | (TaskStatus::Blocked, TaskStatus::Failed)
    )
}

// ── Per-commit guard validation ───────────────────────────────────────────────

/// A validation error describing why a commit or transition failed.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationError {
    /// Index into the commit sequence (0-based, chronological).
    pub commit_index: usize,
    pub message: String,
}

impl ValidationError {
    fn at(commit_index: usize, msg: impl Into<String>) -> Self {
        ValidationError { commit_index, message: msg.into() }
    }
}

/// Validate that `trailers` satisfies the required-field guards for its state.
///
/// Also checks the universal Agent-Id / Session-Id requirement.
/// Returns an empty vec on success.
pub fn validate_commit_guards(index: usize, trailers: &CommitTrailers) -> Vec<ValidationError> {
    let mut errors = vec![];

    // Universal required trailers (schemas.md §7.1 rules 1–3)
    if trailers.agent_id.is_none() {
        errors.push(ValidationError::at(index, "missing required trailer: Agent-Id"));
    }
    if trailers.session_id.is_none() {
        errors.push(ValidationError::at(index, "missing required trailer: Session-Id"));
    }

    let Some(status) = trailers.task_status else {
        return errors;
    };

    match status {
        TaskStatus::Assigned => {
            if trailers.assigned_to.is_none() {
                errors.push(ValidationError::at(index, "ASSIGNED: missing Assigned-To"));
            }
            if trailers.assignment.is_none() {
                errors.push(ValidationError::at(index, "ASSIGNED: missing Assignment"));
            }
            if trailers.scope.is_none() {
                errors.push(ValidationError::at(index, "ASSIGNED: missing Scope"));
            }
            if trailers.dependencies.is_none() {
                errors.push(ValidationError::at(index, "ASSIGNED: missing Dependencies"));
            }
            if trailers.budget.is_none() {
                errors.push(ValidationError::at(index, "ASSIGNED: missing Budget"));
            }
        }
        TaskStatus::Implementing => {
            if trailers.heartbeat.is_none() {
                errors.push(ValidationError::at(index, "IMPLEMENTING: missing Heartbeat"));
            }
        }
        TaskStatus::Completed => {
            if trailers.heartbeat.is_none() {
                errors.push(ValidationError::at(index, "COMPLETED: missing Heartbeat"));
            }
            if trailers.files_changed.is_none() {
                errors.push(ValidationError::at(index, "COMPLETED: missing Files-Changed"));
            }
            if trailers.key_findings.is_empty() {
                errors.push(ValidationError::at(index, "COMPLETED: missing Key-Finding (at least one required)"));
            }
        }
        TaskStatus::Blocked => {
            if trailers.heartbeat.is_none() {
                errors.push(ValidationError::at(index, "BLOCKED: missing Heartbeat"));
            }
            if trailers.blocked_reason.is_none() {
                errors.push(ValidationError::at(index, "BLOCKED: missing Blocked-Reason"));
            }
        }
        TaskStatus::Failed => {
            if trailers.error_category.is_none() {
                errors.push(ValidationError::at(index, "FAILED: missing Error-Category"));
            }
            if trailers.error_retryable.is_none() {
                errors.push(ValidationError::at(index, "FAILED: missing Error-Retryable"));
            }
        }
    }

    errors
}

// ── Branch-level validation ───────────────────────────────────────────────────

/// Validate a full branch commit sequence (chronological, oldest first).
///
/// Applies both per-commit guard checks (§7.1) and branch-level sequence
/// rules (§7.2): first commit ASSIGNED, agent's first IMPLEMENTING, legal
/// transitions, terminal state finality, no duplicate terminals.
///
/// Returns an empty vec iff the sequence is fully valid.
pub fn validate_branch_sequence(commit_messages: &[&str]) -> Vec<ValidationError> {
    let mut errors = vec![];

    if commit_messages.is_empty() {
        return errors;
    }

    let parsed: Vec<CommitTrailers> =
        commit_messages.iter().map(|m| CommitTrailers::parse(m)).collect();

    // Collect state-bearing commits (those with a Task-Status trailer).
    let state_commits: Vec<(usize, TaskStatus)> = parsed
        .iter()
        .enumerate()
        .filter_map(|(i, t)| t.task_status.map(|s| (i, s)))
        .collect();

    if state_commits.is_empty() {
        errors.push(ValidationError::at(0, "branch has no commits with Task-Status"));
        return errors;
    }

    // Rule 9: first commit on the branch must be ASSIGNED.
    let (first_i, first_status) = state_commits[0];
    if first_status != TaskStatus::Assigned {
        errors.push(ValidationError::at(
            first_i,
            format!("first commit must be ASSIGNED, got {:?}", first_status),
        ));
    }

    // Rule 10: agent's first commit (second state-bearing) must be IMPLEMENTING.
    if state_commits.len() >= 2 {
        let (second_i, second_status) = state_commits[1];
        if second_status != TaskStatus::Implementing {
            errors.push(ValidationError::at(
                second_i,
                format!("agent's first commit must be IMPLEMENTING, got {:?}", second_status),
            ));
        }
    }

    // Validate every consecutive state transition.
    for window in state_commits.windows(2) {
        let (_, from) = window[0];
        let (to_i, to) = window[1];
        if !is_valid_transition(from, to) {
            errors.push(ValidationError::at(
                to_i,
                format!("invalid transition: {:?} -> {:?}", from, to),
            ));
        }
    }

    // Rules 11: COMPLETED and FAILED are terminal — at most one of each.
    let completed = state_commits.iter().filter(|(_, s)| *s == TaskStatus::Completed).count();
    let failed = state_commits.iter().filter(|(_, s)| *s == TaskStatus::Failed).count();
    if completed > 1 {
        errors.push(ValidationError::at(
            0,
            format!("branch has {completed} COMPLETED commits; at most one allowed"),
        ));
    }
    if failed > 1 {
        errors.push(ValidationError::at(
            0,
            format!("branch has {failed} FAILED commits; at most one allowed"),
        ));
    }

    // Per-commit guard validation for every state-bearing commit.
    for (i, trailers) in parsed.iter().enumerate() {
        if trailers.task_status.is_some() {
            errors.extend(validate_commit_guards(i, trailers));
        }
    }

    errors
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Fixture helpers ───────────────────────────────────────────────────────

    fn assigned() -> &'static str {
        "task(moss): do a thing\n\n\
         Agent-Id: bitswell\n\
         Session-Id: 00000000-0000-4000-a000-000000000001\n\
         Task-Status: ASSIGNED\n\
         Assigned-To: moss\n\
         Assignment: do-a-thing\n\
         Scope: tests/src/loom/\n\
         Dependencies: none\n\
         Budget: 30\n"
    }

    fn implementing() -> &'static str {
        "chore(loom): begin implementing\n\n\
         Agent-Id: moss\n\
         Session-Id: 00000000-0000-4000-a000-000000000002\n\
         Task-Status: IMPLEMENTING\n\
         Heartbeat: 2026-04-05T00:00:00Z\n"
    }

    fn completed() -> &'static str {
        "test(loom): add state machine tests\n\n\
         Agent-Id: moss\n\
         Session-Id: 00000000-0000-4000-a000-000000000002\n\
         Task-Status: COMPLETED\n\
         Files-Changed: 3\n\
         Key-Finding: state machine implemented\n\
         Heartbeat: 2026-04-05T00:05:00Z\n"
    }

    fn blocked() -> &'static str {
        "chore(loom): blocked -- upstream not ready\n\n\
         Agent-Id: moss\n\
         Session-Id: 00000000-0000-4000-a000-000000000002\n\
         Task-Status: BLOCKED\n\
         Blocked-Reason: waiting on trailers module\n\
         Heartbeat: 2026-04-05T00:02:00Z\n"
    }

    fn resuming() -> &'static str {
        "chore(loom): resume after blocker resolved\n\n\
         Agent-Id: moss\n\
         Session-Id: 00000000-0000-4000-a000-000000000003\n\
         Task-Status: IMPLEMENTING\n\
         Heartbeat: 2026-04-05T00:10:00Z\n"
    }

    fn failed() -> &'static str {
        "chore(loom): failed -- context exhausted\n\n\
         Agent-Id: moss\n\
         Session-Id: 00000000-0000-4000-a000-000000000002\n\
         Task-Status: FAILED\n\
         Error-Category: resource_limit\n\
         Error-Retryable: true\n"
    }

    // ── is_valid_transition ───────────────────────────────────────────────────

    #[test]
    fn valid_transitions_accepted() {
        use TaskStatus::*;
        assert!(is_valid_transition(Assigned, Implementing));
        assert!(is_valid_transition(Implementing, Completed));
        assert!(is_valid_transition(Implementing, Blocked));
        assert!(is_valid_transition(Implementing, Failed));
        assert!(is_valid_transition(Blocked, Implementing));
        assert!(is_valid_transition(Blocked, Failed));
    }

    #[test]
    fn invalid_transitions_rejected() {
        use TaskStatus::*;
        // Terminal states have no outgoing transitions.
        assert!(!is_valid_transition(Completed, Implementing));
        assert!(!is_valid_transition(Completed, Failed));
        assert!(!is_valid_transition(Completed, Blocked));
        assert!(!is_valid_transition(Failed, Implementing));
        assert!(!is_valid_transition(Failed, Completed));
        // BLOCKED cannot jump directly to COMPLETED.
        assert!(!is_valid_transition(Blocked, Completed));
        // ASSIGNED can only go to IMPLEMENTING.
        assert!(!is_valid_transition(Assigned, Completed));
        assert!(!is_valid_transition(Assigned, Blocked));
        assert!(!is_valid_transition(Assigned, Failed));
        // No self-loops.
        assert!(!is_valid_transition(Implementing, Implementing));
        assert!(!is_valid_transition(Assigned, Assigned));
    }

    // ── TaskStatus helpers ────────────────────────────────────────────────────

    #[test]
    fn terminal_states() {
        assert!(TaskStatus::Completed.is_terminal());
        assert!(TaskStatus::Failed.is_terminal());
        assert!(!TaskStatus::Assigned.is_terminal());
        assert!(!TaskStatus::Implementing.is_terminal());
        assert!(!TaskStatus::Blocked.is_terminal());
    }

    #[test]
    fn from_str_roundtrip() {
        for (s, expected) in &[
            ("ASSIGNED", TaskStatus::Assigned),
            ("IMPLEMENTING", TaskStatus::Implementing),
            ("COMPLETED", TaskStatus::Completed),
            ("BLOCKED", TaskStatus::Blocked),
            ("FAILED", TaskStatus::Failed),
        ] {
            assert_eq!(TaskStatus::from_str(s), Some(*expected));
        }
        assert_eq!(TaskStatus::from_str("unknown"), None);
        assert_eq!(TaskStatus::from_str(""), None);
    }

    // ── CommitTrailers::parse ─────────────────────────────────────────────────

    #[test]
    fn parse_assigned_commit() {
        let t = CommitTrailers::parse(assigned());
        assert_eq!(t.task_status, Some(TaskStatus::Assigned));
        assert_eq!(t.agent_id.as_deref(), Some("bitswell"));
        assert_eq!(t.assigned_to.as_deref(), Some("moss"));
        assert_eq!(t.budget, Some(30));
    }

    #[test]
    fn parse_completed_commit() {
        let t = CommitTrailers::parse(completed());
        assert_eq!(t.task_status, Some(TaskStatus::Completed));
        assert_eq!(t.files_changed, Some(3));
        assert_eq!(t.key_findings, vec!["state machine implemented"]);
    }

    #[test]
    fn parse_multiple_key_findings() {
        let msg = "test(loom): done\n\n\
                   Agent-Id: moss\n\
                   Session-Id: 00000000-0000-4000-a000-000000000002\n\
                   Task-Status: COMPLETED\n\
                   Files-Changed: 2\n\
                   Key-Finding: first finding\n\
                   Key-Finding: second finding\n\
                   Heartbeat: 2026-04-05T00:05:00Z\n";
        let t = CommitTrailers::parse(msg);
        assert_eq!(t.key_findings.len(), 2);
    }

    // ── validate_commit_guards ────────────────────────────────────────────────

    #[test]
    fn guard_valid_assigned() {
        let t = CommitTrailers::parse(assigned());
        assert!(validate_commit_guards(0, &t).is_empty());
    }

    #[test]
    fn guard_valid_implementing() {
        let t = CommitTrailers::parse(implementing());
        assert!(validate_commit_guards(1, &t).is_empty());
    }

    #[test]
    fn guard_valid_completed() {
        let t = CommitTrailers::parse(completed());
        assert!(validate_commit_guards(2, &t).is_empty());
    }

    #[test]
    fn guard_valid_blocked() {
        let t = CommitTrailers::parse(blocked());
        assert!(validate_commit_guards(2, &t).is_empty());
    }

    #[test]
    fn guard_valid_failed() {
        let t = CommitTrailers::parse(failed());
        assert!(validate_commit_guards(2, &t).is_empty());
    }

    #[test]
    fn guard_completed_missing_key_finding() {
        let msg = "test(loom): done\n\n\
                   Agent-Id: moss\n\
                   Session-Id: 00000000-0000-4000-a000-000000000002\n\
                   Task-Status: COMPLETED\n\
                   Files-Changed: 1\n\
                   Heartbeat: 2026-04-05T00:05:00Z\n";
        let t = CommitTrailers::parse(msg);
        let errs = validate_commit_guards(0, &t);
        assert!(errs.iter().any(|e| e.message.contains("Key-Finding")));
    }

    #[test]
    fn guard_completed_missing_files_changed() {
        let msg = "test(loom): done\n\n\
                   Agent-Id: moss\n\
                   Session-Id: 00000000-0000-4000-a000-000000000002\n\
                   Task-Status: COMPLETED\n\
                   Key-Finding: something\n\
                   Heartbeat: 2026-04-05T00:05:00Z\n";
        let t = CommitTrailers::parse(msg);
        let errs = validate_commit_guards(0, &t);
        assert!(errs.iter().any(|e| e.message.contains("Files-Changed")));
    }

    #[test]
    fn guard_blocked_missing_reason() {
        let msg = "chore(loom): blocked\n\n\
                   Agent-Id: moss\n\
                   Session-Id: 00000000-0000-4000-a000-000000000002\n\
                   Task-Status: BLOCKED\n\
                   Heartbeat: 2026-04-05T00:02:00Z\n";
        let t = CommitTrailers::parse(msg);
        let errs = validate_commit_guards(0, &t);
        assert!(errs.iter().any(|e| e.message.contains("Blocked-Reason")));
    }

    #[test]
    fn guard_failed_missing_error_trailers() {
        let msg = "chore(loom): failed\n\n\
                   Agent-Id: moss\n\
                   Session-Id: 00000000-0000-4000-a000-000000000002\n\
                   Task-Status: FAILED\n";
        let t = CommitTrailers::parse(msg);
        let errs = validate_commit_guards(0, &t);
        assert!(errs.iter().any(|e| e.message.contains("Error-Category")));
        assert!(errs.iter().any(|e| e.message.contains("Error-Retryable")));
    }

    #[test]
    fn guard_missing_agent_id() {
        let msg = "chore(loom): work\n\n\
                   Session-Id: 00000000-0000-4000-a000-000000000002\n\
                   Task-Status: IMPLEMENTING\n\
                   Heartbeat: 2026-04-05T00:00:00Z\n";
        let t = CommitTrailers::parse(msg);
        let errs = validate_commit_guards(0, &t);
        assert!(errs.iter().any(|e| e.message.contains("Agent-Id")));
    }

    // ── validate_branch_sequence — valid paths ────────────────────────────────

    #[test]
    fn valid_happy_path() {
        let seq = [assigned(), implementing(), completed()];
        assert!(validate_branch_sequence(&seq).is_empty());
    }

    #[test]
    fn valid_blocked_then_resume_then_complete() {
        let seq = [assigned(), implementing(), blocked(), resuming(), completed()];
        assert!(validate_branch_sequence(&seq).is_empty());
    }

    #[test]
    fn valid_implementing_to_failed() {
        let seq = [assigned(), implementing(), failed()];
        assert!(validate_branch_sequence(&seq).is_empty());
    }

    #[test]
    fn valid_blocked_to_failed() {
        let seq = [assigned(), implementing(), blocked(), failed()];
        assert!(validate_branch_sequence(&seq).is_empty());
    }

    /// Non-state-bearing commits between state commits are allowed (work commits).
    #[test]
    fn valid_intermediate_work_commits() {
        let work = "test(loom): add tests\n\n\
                    Agent-Id: moss\n\
                    Session-Id: 00000000-0000-4000-a000-000000000002\n\
                    Heartbeat: 2026-04-05T00:03:00Z\n";
        let seq = [assigned(), implementing(), work, work, completed()];
        assert!(validate_branch_sequence(&seq).is_empty());
    }

    // ── validate_branch_sequence — invalid paths ──────────────────────────────

    #[test]
    fn invalid_missing_assigned_first() {
        let seq = [implementing(), completed()];
        let errs = validate_branch_sequence(&seq);
        assert!(errs.iter().any(|e| e.message.contains("ASSIGNED")));
    }

    #[test]
    fn invalid_skip_implementing() {
        // ASSIGNED directly to COMPLETED — skips IMPLEMENTING.
        let seq = [assigned(), completed()];
        let errs = validate_branch_sequence(&seq);
        // Second state commit must be IMPLEMENTING.
        assert!(errs.iter().any(|e| e.message.contains("IMPLEMENTING")));
        // And the transition ASSIGNED -> COMPLETED is invalid.
        assert!(errs.iter().any(|e| e.message.contains("invalid transition")));
    }

    #[test]
    fn invalid_after_completed() {
        // COMPLETED is terminal — no further Task-Status commits allowed.
        let seq = [assigned(), implementing(), completed(), implementing()];
        let errs = validate_branch_sequence(&seq);
        assert!(errs.iter().any(|e| e.message.contains("invalid transition")));
    }

    #[test]
    fn invalid_blocked_to_completed_directly() {
        // Must resume IMPLEMENTING before COMPLETED.
        let seq = [assigned(), implementing(), blocked(), completed()];
        let errs = validate_branch_sequence(&seq);
        assert!(errs.iter().any(|e| e.message.contains("invalid transition")));
    }

    #[test]
    fn invalid_assigned_to_blocked() {
        // Cannot jump from ASSIGNED to BLOCKED.
        let seq = [assigned(), blocked()];
        let errs = validate_branch_sequence(&seq);
        assert!(!errs.is_empty());
    }

    #[test]
    fn invalid_duplicate_completed() {
        let seq = [assigned(), implementing(), completed(), completed()];
        let errs = validate_branch_sequence(&seq);
        assert!(
            errs.iter().any(|e| e.message.contains("COMPLETED")),
            "expected duplicate-COMPLETED error, got: {errs:?}"
        );
    }

    #[test]
    fn invalid_duplicate_failed() {
        let seq = [assigned(), implementing(), failed(), failed()];
        let errs = validate_branch_sequence(&seq);
        assert!(
            errs.iter().any(|e| e.message.contains("FAILED")),
            "expected duplicate-FAILED error, got: {errs:?}"
        );
    }

    #[test]
    fn invalid_failed_to_implementing() {
        // FAILED is terminal.
        let seq = [assigned(), implementing(), failed(), implementing()];
        let errs = validate_branch_sequence(&seq);
        assert!(errs.iter().any(|e| e.message.contains("invalid transition")));
    }

    #[test]
    fn empty_sequence_is_clean() {
        assert!(validate_branch_sequence(&[]).is_empty());
    }

    #[test]
    fn sequence_with_no_status_trailers() {
        let work = "test(loom): some work\n\nAgent-Id: moss\nSession-Id: s1\n";
        let errs = validate_branch_sequence(&[work]);
        assert!(errs.iter().any(|e| e.message.contains("no commits with Task-Status")));
    }
}
