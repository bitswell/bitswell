//! LOOM protocol trailer parser and validator.
//!
//! Parses conventional commit messages into structured data and validates
//! them against the LOOM protocol schema (v2.0.0).

// ── Types ────────────────────────────────────────────────────────────────────

/// A parsed conventional commit message.
#[derive(Debug, Clone, PartialEq)]
pub struct CommitMessage {
    pub commit_type: String,
    pub scope: Option<String>,
    pub subject: String,
    pub body: Option<String>,
    /// Trailers in document order. Multiple entries with the same key are allowed.
    pub trailers: Vec<(String, String)>,
}

impl CommitMessage {
    /// First value for a trailer key (case-insensitive).
    pub fn trailer(&self, key: &str) -> Option<&str> {
        self.trailers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.as_str())
    }

    /// All values for a trailer key (case-insensitive).
    pub fn trailers_all(&self, key: &str) -> Vec<&str> {
        self.trailers
            .iter()
            .filter(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.as_str())
            .collect()
    }
}

/// A validation error.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationError {
    pub message: String,
}

impl ValidationError {
    fn new(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
        }
    }
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

/// Task lifecycle states.
#[derive(Debug, Clone, PartialEq)]
pub enum TaskStatus {
    Assigned,
    Implementing,
    Completed,
    Blocked,
    Failed,
}

impl TaskStatus {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim() {
            "ASSIGNED" => Some(Self::Assigned),
            "IMPLEMENTING" => Some(Self::Implementing),
            "COMPLETED" => Some(Self::Completed),
            "BLOCKED" => Some(Self::Blocked),
            "FAILED" => Some(Self::Failed),
            _ => None,
        }
    }
}

// ── Parser ───────────────────────────────────────────────────────────────────

/// Parse a raw commit message string into a [`CommitMessage`].
///
/// Returns `Err` if the first line does not match the conventional commit format.
pub fn parse_commit(raw: &str) -> Result<CommitMessage, String> {
    let mut lines = raw.lines();
    let header = lines.next().unwrap_or("").trim();
    let (commit_type, scope, subject) = parse_header(header)?;
    let rest: Vec<&str> = lines.collect();
    let (body, trailers) = split_body_trailers(&rest);
    Ok(CommitMessage {
        commit_type,
        scope,
        subject,
        body,
        trailers,
    })
}

fn parse_header(header: &str) -> Result<(String, Option<String>, String), String> {
    let colon_pos = header
        .find(':')
        .ok_or_else(|| format!("header has no colon: {header:?}"))?;

    let type_scope = &header[..colon_pos];
    let subject = header[colon_pos + 1..].trim();

    let (commit_type, scope) = if let (Some(open), Some(close)) =
        (type_scope.find('('), type_scope.rfind(')'))
    {
        (
            type_scope[..open].trim().to_string(),
            Some(type_scope[open + 1..close].trim().to_string()),
        )
    } else {
        (type_scope.trim().to_string(), None)
    };

    if commit_type.is_empty() {
        return Err("commit type is empty".into());
    }
    if subject.is_empty() {
        return Err("subject is empty".into());
    }
    Ok((commit_type, scope, subject.to_string()))
}

/// Split lines after the header into (body, trailers).
///
/// The last non-empty block is treated as trailers if every line in it
/// matches the `Key: value` trailer format.
fn split_body_trailers(lines: &[&str]) -> (Option<String>, Vec<(String, String)>) {
    let mut blocks: Vec<Vec<&str>> = vec![];
    let mut current: Vec<&str> = vec![];
    for &line in lines {
        if line.trim().is_empty() {
            if !current.is_empty() {
                blocks.push(std::mem::take(&mut current));
            }
        } else {
            current.push(line);
        }
    }
    if !current.is_empty() {
        blocks.push(current);
    }

    let trailer_block_idx = blocks
        .last()
        .filter(|block| !block.is_empty() && block.iter().all(|l| looks_like_trailer(l)))
        .map(|_| blocks.len() - 1);

    let trailers: Vec<(String, String)> = match trailer_block_idx {
        Some(i) => blocks[i]
            .iter()
            .filter_map(|l| parse_trailer_line(l))
            .collect(),
        None => vec![],
    };

    let body_blocks = match trailer_block_idx {
        Some(i) => &blocks[..i],
        None => &blocks[..],
    };

    let body = if body_blocks.is_empty() {
        None
    } else {
        Some(
            body_blocks
                .iter()
                .map(|block| block.join("\n"))
                .collect::<Vec<_>>()
                .join("\n\n"),
        )
    };

    (body, trailers)
}

fn looks_like_trailer(line: &str) -> bool {
    if let Some(pos) = line.find(':') {
        let key = &line[..pos];
        !key.is_empty()
            && !key.contains(' ')
            && key.chars().all(|c| c.is_alphanumeric() || c == '-')
            && line[pos + 1..].starts_with(' ')
    } else {
        false
    }
}

fn parse_trailer_line(line: &str) -> Option<(String, String)> {
    let pos = line.find(':')?;
    let key = line[..pos].trim().to_string();
    let value = line[pos + 1..].trim().to_string();
    if key.is_empty() {
        return None;
    }
    Some((key, value))
}

// ── Per-commit validation ─────────────────────────────────────────────────────

/// Validate a single commit message against the LOOM protocol.
///
/// Returns a list of errors (empty = valid).
pub fn validate_commit(msg: &CommitMessage) -> Vec<ValidationError> {
    let mut errors = vec![];

    // Agent-Id: required, must be kebab-case
    match msg.trailer("Agent-Id") {
        None => errors.push(ValidationError::new("missing required trailer: Agent-Id")),
        Some(id) => {
            if !is_kebab_case(id) {
                errors.push(ValidationError::new(format!(
                    "Agent-Id {id:?} is not kebab-case ([a-z0-9]+(-[a-z0-9]+)*)"
                )));
            }
        }
    }

    // Session-Id: required, must be UUID v4
    match msg.trailer("Session-Id") {
        None => errors.push(ValidationError::new("missing required trailer: Session-Id")),
        Some(sid) => {
            if !is_uuid_v4(sid) {
                errors.push(ValidationError::new(format!(
                    "Session-Id {sid:?} is not a valid UUID v4"
                )));
            }
        }
    }

    // Heartbeat: optional presence, but must be RFC3339 if present
    if let Some(hb) = msg.trailer("Heartbeat") {
        if !is_rfc3339(hb) {
            errors.push(ValidationError::new(format!(
                "Heartbeat {hb:?} is not a valid RFC3339 timestamp"
            )));
        }
    }

    // Task-Status: must be valid enum value if present
    let task_status = match msg.trailer("Task-Status") {
        None => None,
        Some(s) => match TaskStatus::parse(s) {
            Some(ts) => Some(ts),
            None => {
                errors.push(ValidationError::new(format!(
                    "Task-Status {s:?} must be one of: ASSIGNED, IMPLEMENTING, COMPLETED, BLOCKED, FAILED"
                )));
                None
            }
        },
    };

    // State-conditional requirements
    match &task_status {
        Some(TaskStatus::Assigned) => {
            for required in &["Assigned-To", "Assignment", "Scope", "Dependencies", "Budget"] {
                if msg.trailer(required).is_none() {
                    errors.push(ValidationError::new(format!(
                        "Task-Status ASSIGNED requires trailer: {required}"
                    )));
                }
            }
        }
        Some(TaskStatus::Implementing) => {
            if msg.trailer("Heartbeat").is_none() {
                errors.push(ValidationError::new(
                    "Task-Status IMPLEMENTING requires trailer: Heartbeat",
                ));
            }
        }
        Some(TaskStatus::Completed) => {
            match msg.trailer("Files-Changed") {
                None => errors.push(ValidationError::new(
                    "Task-Status COMPLETED requires trailer: Files-Changed",
                )),
                Some(fc) => {
                    if fc.parse::<u64>().is_err() {
                        errors.push(ValidationError::new(format!(
                            "Files-Changed {fc:?} must be a non-negative integer"
                        )));
                    }
                }
            }
            if msg.trailers_all("Key-Finding").is_empty() {
                errors.push(ValidationError::new(
                    "Task-Status COMPLETED requires at least one Key-Finding trailer",
                ));
            }
            if msg.trailer("Heartbeat").is_none() {
                errors.push(ValidationError::new(
                    "Task-Status COMPLETED requires trailer: Heartbeat",
                ));
            }
        }
        Some(TaskStatus::Blocked) => {
            if msg.trailer("Blocked-Reason").is_none() {
                errors.push(ValidationError::new(
                    "Task-Status BLOCKED requires trailer: Blocked-Reason",
                ));
            }
            if msg.trailer("Heartbeat").is_none() {
                errors.push(ValidationError::new(
                    "Task-Status BLOCKED requires trailer: Heartbeat",
                ));
            }
        }
        Some(TaskStatus::Failed) => {
            match msg.trailer("Error-Category") {
                None => errors.push(ValidationError::new(
                    "Task-Status FAILED requires trailer: Error-Category",
                )),
                Some(ec) => {
                    const VALID: &[&str] =
                        &["task_unclear", "blocked", "resource_limit", "conflict", "internal"];
                    if !VALID.contains(&ec) {
                        errors.push(ValidationError::new(format!(
                            "Error-Category {ec:?} must be one of: task_unclear, blocked, resource_limit, conflict, internal"
                        )));
                    }
                }
            }
            match msg.trailer("Error-Retryable") {
                None => errors.push(ValidationError::new(
                    "Task-Status FAILED requires trailer: Error-Retryable",
                )),
                Some(er) => {
                    if er != "true" && er != "false" {
                        errors.push(ValidationError::new(format!(
                            "Error-Retryable {er:?} must be \"true\" or \"false\""
                        )));
                    }
                }
            }
        }
        None => {}
    }

    errors
}

// ── Branch validation ─────────────────────────────────────────────────────────

/// Validate a branch name against the LOOM naming convention.
///
/// Valid: `loom/<agent>-<slug>` where the suffix is kebab-case and total <= 63 chars.
pub fn validate_branch(branch: &str) -> Vec<ValidationError> {
    let mut errors = vec![];

    if branch.len() > 63 {
        errors.push(ValidationError::new(format!(
            "branch name exceeds 63 characters (len={}): {branch:?}",
            branch.len()
        )));
    }

    let Some(rest) = branch.strip_prefix("loom/") else {
        errors.push(ValidationError::new(format!(
            "branch {branch:?} does not start with \"loom/\""
        )));
        return errors;
    };

    if !is_kebab_case(rest) {
        errors.push(ValidationError::new(format!(
            "branch suffix {rest:?} is not kebab-case ([a-z0-9]+(-[a-z0-9]+)*)"
        )));
    }

    if !rest.contains('-') {
        errors.push(ValidationError::new(format!(
            "branch suffix {rest:?} must be <agent>-<slug> (missing hyphen)"
        )));
    }

    errors
}

// ── Branch-history (sequence) validation ─────────────────────────────────────

/// Validate a sequence of commits on a branch against branch-level LOOM rules.
///
/// `commits` must be in chronological order (oldest first).
pub fn validate_branch_history(commits: &[CommitMessage]) -> Vec<ValidationError> {
    let mut errors = vec![];

    if commits.is_empty() {
        return errors;
    }

    // Rule 9: first commit must be ASSIGNED
    if commits[0].trailer("Task-Status") != Some("ASSIGNED") {
        errors.push(ValidationError::new(format!(
            "first commit must have Task-Status: ASSIGNED, got: {:?}",
            commits[0].trailer("Task-Status")
        )));
    }

    let mut seen_terminal = false;
    let mut prev_status: Option<TaskStatus> = None;

    for (i, commit) in commits.iter().enumerate() {
        let status = commit
            .trailer("Task-Status")
            .and_then(TaskStatus::parse);

        // Rule 12: no Task-Status commits after a terminal state
        if seen_terminal {
            if let Some(s) = commit.trailer("Task-Status") {
                errors.push(ValidationError::new(format!(
                    "commit {} carries Task-Status: {s:?} after a terminal state",
                    i + 1
                )));
            }
        }

        if let Some(ref current) = status {
            // State machine transition
            if let Some(ref prev) = prev_status {
                if !is_valid_transition(prev, current) {
                    errors.push(ValidationError::new(format!(
                        "invalid state transition at commit {}: {:?} -> {:?}",
                        i + 1,
                        prev,
                        current
                    )));
                }
            }

            if matches!(current, TaskStatus::Completed | TaskStatus::Failed) {
                seen_terminal = true;
            }

            prev_status = Some(current.clone());
        }
    }

    errors
}

fn is_valid_transition(from: &TaskStatus, to: &TaskStatus) -> bool {
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

// ── Format validators ─────────────────────────────────────────────────────────

/// Returns true if `s` matches `[a-z0-9]+(-[a-z0-9]+)*`.
pub fn is_kebab_case(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    s.split('-')
        .all(|seg| !seg.is_empty() && seg.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()))
}

/// Returns true if `s` is a valid UUID v4 (`xxxxxxxx-xxxx-4xxx-[89ab]xxx-xxxxxxxxxxxx`).
pub fn is_uuid_v4(s: &str) -> bool {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 5 {
        return false;
    }
    let expected_lengths = [8usize, 4, 4, 4, 12];
    for (part, &len) in parts.iter().zip(expected_lengths.iter()) {
        if part.len() != len || !part.chars().all(|c| c.is_ascii_hexdigit()) {
            return false;
        }
    }
    // Third group must start with '4' (version 4)
    if !parts[2].starts_with('4') {
        return false;
    }
    // Fourth group must start with 8, 9, a, or b (variant bits)
    matches!(parts[3].chars().next(), Some('8' | '9' | 'a' | 'b' | 'A' | 'B'))
}

/// Returns true if `s` is a valid RFC3339 timestamp.
///
/// Accepts `YYYY-MM-DDTHH:MM:SSZ`, `YYYY-MM-DDTHH:MM:SS.sssZ`,
/// and `±HH:MM` timezone offsets.
pub fn is_rfc3339(s: &str) -> bool {
    if s.len() < 20 {
        return false;
    }
    let b = s.as_bytes();
    if !is_digit4(&b[0..4])
        || b[4] != b'-'
        || !is_digit2(&b[5..7])
        || b[7] != b'-'
        || !is_digit2(&b[8..10])
        || (b[10] != b'T' && b[10] != b't')
        || !is_digit2(&b[11..13])
        || b[13] != b':'
        || !is_digit2(&b[14..16])
        || b[16] != b':'
        || !is_digit2(&b[17..19])
    {
        return false;
    }

    let rest = &s[19..];
    let after_frac = if rest.starts_with('.') {
        let digits_end = rest[1..]
            .find(|c: char| !c.is_ascii_digit())
            .map(|n| n + 1)
            .unwrap_or(rest.len());
        &rest[digits_end..]
    } else {
        rest
    };

    after_frac == "Z"
        || after_frac == "z"
        || (after_frac.len() == 6
            && (after_frac.starts_with('+') || after_frac.starts_with('-'))
            && is_digit2(after_frac[1..3].as_bytes())
            && after_frac.as_bytes()[3] == b':'
            && is_digit2(after_frac[4..6].as_bytes()))
}

fn is_digit4(b: &[u8]) -> bool {
    b.len() >= 4 && b[..4].iter().all(|c| c.is_ascii_digit())
}

fn is_digit2(b: &[u8]) -> bool {
    b.len() >= 2 && b[..2].iter().all(|c| c.is_ascii_digit())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Parser tests ──

    #[test]
    fn test_parse_simple_work_commit() {
        let raw = "feat(loom): add trailer parser\n\nInitial implementation.\n\nAgent-Id: ratchet\nSession-Id: f8309ae8-ac76-4766-8926-362cdd06d04b\nHeartbeat: 2026-04-05T12:00:00Z";
        let msg = parse_commit(raw).unwrap();
        assert_eq!(msg.commit_type, "feat");
        assert_eq!(msg.scope, Some("loom".into()));
        assert_eq!(msg.subject, "add trailer parser");
        assert_eq!(msg.body, Some("Initial implementation.".into()));
        assert_eq!(msg.trailer("Agent-Id"), Some("ratchet"));
        assert_eq!(
            msg.trailer("Session-Id"),
            Some("f8309ae8-ac76-4766-8926-362cdd06d04b")
        );
        assert_eq!(msg.trailer("Heartbeat"), Some("2026-04-05T12:00:00Z"));
    }

    #[test]
    fn test_parse_assigned_commit() {
        let raw = "task(ratchet): scaffold loom plugin directory structure\n\nCreate the initial loom/ directory.\n\nAgent-Id: bitswell\nSession-Id: f8309ae8-ac76-4766-8926-362cdd06d04b\nTask-Status: ASSIGNED\nAssigned-To: ratchet\nAssignment: plugin-scaffold\nScope: loom/**\nDependencies: none\nBudget: 100000";
        let msg = parse_commit(raw).unwrap();
        assert_eq!(msg.commit_type, "task");
        assert_eq!(msg.scope, Some("ratchet".into()));
        assert_eq!(msg.trailer("Task-Status"), Some("ASSIGNED"));
        assert_eq!(msg.trailer("Assigned-To"), Some("ratchet"));
        assert_eq!(msg.trailer("Assignment"), Some("plugin-scaffold"));
        assert_eq!(msg.trailer("Budget"), Some("100000"));
    }

    #[test]
    fn test_parse_completed_commit_multiple_key_findings() {
        let raw = "feat(loom): implement trailer parser\n\nAgent-Id: ratchet\nSession-Id: f8309ae8-ac76-4766-8926-362cdd06d04b\nTask-Status: COMPLETED\nFiles-Changed: 3\nKey-Finding: parser handles multi-line bodies correctly\nKey-Finding: UUID v4 validation covers all variant bits\nHeartbeat: 2026-04-05T14:00:00Z";
        let msg = parse_commit(raw).unwrap();
        assert_eq!(msg.trailer("Task-Status"), Some("COMPLETED"));
        assert_eq!(msg.trailer("Files-Changed"), Some("3"));
        let findings = msg.trailers_all("Key-Finding");
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0], "parser handles multi-line bodies correctly");
    }

    #[test]
    fn test_parse_no_scope() {
        let raw = "chore: cleanup\n\nAgent-Id: ratchet\nSession-Id: f8309ae8-ac76-4766-8926-362cdd06d04b";
        let msg = parse_commit(raw).unwrap();
        assert_eq!(msg.commit_type, "chore");
        assert_eq!(msg.scope, None);
    }

    #[test]
    fn test_parse_invalid_header() {
        assert!(parse_commit("no colon here").is_err());
    }

    // ── Commit validation tests ──

    fn valid_implementing() -> CommitMessage {
        parse_commit(
            "chore(loom): begin implementing trailer parser\n\nAgent-Id: ratchet\nSession-Id: f8309ae8-ac76-4766-8926-362cdd06d04b\nTask-Status: IMPLEMENTING\nHeartbeat: 2026-04-05T12:00:00Z",
        )
        .unwrap()
    }

    fn valid_completed() -> CommitMessage {
        parse_commit(
            "feat(loom): implement trailer parser\n\nAgent-Id: ratchet\nSession-Id: f8309ae8-ac76-4766-8926-362cdd06d04b\nTask-Status: COMPLETED\nFiles-Changed: 2\nKey-Finding: trailer parser and validator implemented\nHeartbeat: 2026-04-05T14:00:00Z",
        )
        .unwrap()
    }

    #[test]
    fn test_validate_valid_implementing() {
        assert_eq!(validate_commit(&valid_implementing()), vec![]);
    }

    #[test]
    fn test_validate_valid_completed() {
        assert_eq!(validate_commit(&valid_completed()), vec![]);
    }

    #[test]
    fn test_validate_valid_assigned() {
        let msg = parse_commit("task(ratchet): scaffold loom plugin\n\nFull description.\n\nAgent-Id: bitswell\nSession-Id: f8309ae8-ac76-4766-8926-362cdd06d04b\nTask-Status: ASSIGNED\nAssigned-To: ratchet\nAssignment: plugin-scaffold\nScope: loom/**\nDependencies: none\nBudget: 100000").unwrap();
        assert_eq!(validate_commit(&msg), vec![]);
    }

    #[test]
    fn test_validate_valid_blocked() {
        let msg = parse_commit("chore(loom): blocked -- missing dependency\n\nAgent-Id: ratchet\nSession-Id: f8309ae8-ac76-4766-8926-362cdd06d04b\nTask-Status: BLOCKED\nBlocked-Reason: dependency loom/moss-loom-worker is not COMPLETED\nHeartbeat: 2026-04-05T13:00:00Z").unwrap();
        assert_eq!(validate_commit(&msg), vec![]);
    }

    #[test]
    fn test_validate_valid_failed() {
        let msg = parse_commit("chore(loom): failed -- unrecoverable error\n\nAgent-Id: ratchet\nSession-Id: f8309ae8-ac76-4766-8926-362cdd06d04b\nTask-Status: FAILED\nError-Category: internal\nError-Retryable: false").unwrap();
        assert_eq!(validate_commit(&msg), vec![]);
    }

    #[test]
    fn test_validate_missing_agent_id() {
        let msg = parse_commit("feat(loom): add something\n\nSession-Id: f8309ae8-ac76-4766-8926-362cdd06d04b").unwrap();
        let errors = validate_commit(&msg);
        assert!(
            errors.iter().any(|e| e.message.contains("Agent-Id")),
            "expected Agent-Id error, got: {errors:?}"
        );
    }

    #[test]
    fn test_validate_missing_session_id() {
        let msg = parse_commit("feat(loom): add something\n\nAgent-Id: ratchet").unwrap();
        let errors = validate_commit(&msg);
        assert!(
            errors.iter().any(|e| e.message.contains("Session-Id")),
            "expected Session-Id error, got: {errors:?}"
        );
    }

    #[test]
    fn test_validate_invalid_agent_id_format() {
        let msg = parse_commit("feat(loom): add something\n\nAgent-Id: My Agent\nSession-Id: f8309ae8-ac76-4766-8926-362cdd06d04b").unwrap();
        let errors = validate_commit(&msg);
        assert!(
            errors.iter().any(|e| e.message.contains("kebab-case")),
            "expected kebab-case error, got: {errors:?}"
        );
    }

    #[test]
    fn test_validate_invalid_session_id_format() {
        let msg = parse_commit("feat(loom): add something\n\nAgent-Id: ratchet\nSession-Id: not-a-uuid").unwrap();
        let errors = validate_commit(&msg);
        assert!(
            errors.iter().any(|e| e.message.contains("UUID v4")),
            "expected UUID v4 error, got: {errors:?}"
        );
    }

    #[test]
    fn test_validate_invalid_task_status() {
        let msg = parse_commit("feat(loom): add something\n\nAgent-Id: ratchet\nSession-Id: f8309ae8-ac76-4766-8926-362cdd06d04b\nTask-Status: RUNNING").unwrap();
        let errors = validate_commit(&msg);
        assert!(
            errors.iter().any(|e| e.message.contains("Task-Status")),
            "expected Task-Status error, got: {errors:?}"
        );
    }

    #[test]
    fn test_validate_implementing_missing_heartbeat() {
        let msg = parse_commit("chore(loom): begin\n\nAgent-Id: ratchet\nSession-Id: f8309ae8-ac76-4766-8926-362cdd06d04b\nTask-Status: IMPLEMENTING").unwrap();
        let errors = validate_commit(&msg);
        assert!(
            errors.iter().any(|e| e.message.contains("Heartbeat")),
            "expected Heartbeat error, got: {errors:?}"
        );
    }

    #[test]
    fn test_validate_completed_missing_key_finding() {
        let msg = parse_commit("feat(loom): done\n\nAgent-Id: ratchet\nSession-Id: f8309ae8-ac76-4766-8926-362cdd06d04b\nTask-Status: COMPLETED\nFiles-Changed: 1\nHeartbeat: 2026-04-05T14:00:00Z").unwrap();
        let errors = validate_commit(&msg);
        assert!(
            errors.iter().any(|e| e.message.contains("Key-Finding")),
            "expected Key-Finding error, got: {errors:?}"
        );
    }

    #[test]
    fn test_validate_completed_missing_files_changed() {
        let msg = parse_commit("feat(loom): done\n\nAgent-Id: ratchet\nSession-Id: f8309ae8-ac76-4766-8926-362cdd06d04b\nTask-Status: COMPLETED\nKey-Finding: something\nHeartbeat: 2026-04-05T14:00:00Z").unwrap();
        let errors = validate_commit(&msg);
        assert!(
            errors.iter().any(|e| e.message.contains("Files-Changed")),
            "expected Files-Changed error, got: {errors:?}"
        );
    }

    #[test]
    fn test_validate_blocked_missing_reason() {
        let msg = parse_commit("chore(loom): blocked\n\nAgent-Id: ratchet\nSession-Id: f8309ae8-ac76-4766-8926-362cdd06d04b\nTask-Status: BLOCKED\nHeartbeat: 2026-04-05T13:00:00Z").unwrap();
        let errors = validate_commit(&msg);
        assert!(
            errors.iter().any(|e| e.message.contains("Blocked-Reason")),
            "expected Blocked-Reason error, got: {errors:?}"
        );
    }

    #[test]
    fn test_validate_failed_missing_error_trailers() {
        let msg = parse_commit("chore(loom): failed\n\nAgent-Id: ratchet\nSession-Id: f8309ae8-ac76-4766-8926-362cdd06d04b\nTask-Status: FAILED").unwrap();
        let errors = validate_commit(&msg);
        assert!(
            errors.iter().any(|e| e.message.contains("Error-Category")),
            "expected Error-Category error"
        );
        assert!(
            errors.iter().any(|e| e.message.contains("Error-Retryable")),
            "expected Error-Retryable error"
        );
    }

    #[test]
    fn test_validate_failed_invalid_error_category() {
        let msg = parse_commit("chore(loom): failed\n\nAgent-Id: ratchet\nSession-Id: f8309ae8-ac76-4766-8926-362cdd06d04b\nTask-Status: FAILED\nError-Category: unknown_type\nError-Retryable: false").unwrap();
        let errors = validate_commit(&msg);
        assert!(
            errors.iter().any(|e| e.message.contains("Error-Category")),
            "expected Error-Category validation error, got: {errors:?}"
        );
    }

    #[test]
    fn test_validate_invalid_heartbeat_format() {
        let msg = parse_commit("feat(loom): work\n\nAgent-Id: ratchet\nSession-Id: f8309ae8-ac76-4766-8926-362cdd06d04b\nHeartbeat: 2026-04-05 12:00:00").unwrap();
        let errors = validate_commit(&msg);
        assert!(
            errors.iter().any(|e| e.message.contains("RFC3339")),
            "expected RFC3339 error, got: {errors:?}"
        );
    }

    #[test]
    fn test_validate_assigned_missing_required_trailers() {
        let msg = parse_commit("task(ratchet): do work\n\nAgent-Id: bitswell\nSession-Id: f8309ae8-ac76-4766-8926-362cdd06d04b\nTask-Status: ASSIGNED").unwrap();
        let errors = validate_commit(&msg);
        let missing: Vec<_> = errors.iter().map(|e| e.message.as_str()).collect();
        for required in &["Assigned-To", "Assignment", "Scope", "Dependencies", "Budget"] {
            assert!(
                missing.iter().any(|m| m.contains(required)),
                "expected missing {required} error, got: {missing:?}"
            );
        }
    }

    // ── Branch name validation tests ──

    #[test]
    fn test_validate_branch_valid() {
        assert_eq!(validate_branch("loom/ratchet-plugin-scaffold"), vec![]);
        assert_eq!(validate_branch("loom/moss-loom-worker"), vec![]);
        assert_eq!(validate_branch("loom/test-1"), vec![]);
    }

    #[test]
    fn test_validate_branch_no_loom_prefix() {
        let errors = validate_branch("ratchet-plugin-scaffold");
        assert!(
            errors.iter().any(|e| e.message.contains("loom/")),
            "expected loom/ prefix error, got: {errors:?}"
        );
    }

    #[test]
    fn test_validate_branch_no_hyphen() {
        let errors = validate_branch("loom/ratchet");
        assert!(
            errors.iter().any(|e| e.message.contains("hyphen")),
            "expected hyphen error, got: {errors:?}"
        );
    }

    #[test]
    fn test_validate_branch_uppercase() {
        let errors = validate_branch("loom/Ratchet-task");
        assert!(!errors.is_empty(), "expected error for uppercase");
    }

    #[test]
    fn test_validate_branch_too_long() {
        let branch = format!("loom/ratchet-{}", "a".repeat(60));
        let errors = validate_branch(&branch);
        assert!(
            errors.iter().any(|e| e.message.contains("63")),
            "expected length error, got: {errors:?}"
        );
    }

    // ── Branch history validation tests ──

    fn assigned_commit() -> CommitMessage {
        parse_commit("task(ratchet): do work\n\nFull description.\n\nAgent-Id: bitswell\nSession-Id: f8309ae8-ac76-4766-8926-362cdd06d04b\nTask-Status: ASSIGNED\nAssigned-To: ratchet\nAssignment: do-work\nScope: loom/**\nDependencies: none\nBudget: 100000").unwrap()
    }

    #[test]
    fn test_branch_history_valid_full_sequence() {
        let commits = vec![
            assigned_commit(),
            valid_implementing(),
            valid_completed(),
        ];
        assert_eq!(validate_branch_history(&commits), vec![]);
    }

    #[test]
    fn test_branch_history_valid_with_blocked() {
        let blocked = parse_commit("chore(loom): blocked\n\nAgent-Id: ratchet\nSession-Id: f8309ae8-ac76-4766-8926-362cdd06d04b\nTask-Status: BLOCKED\nBlocked-Reason: waiting on dep\nHeartbeat: 2026-04-05T13:00:00Z").unwrap();
        let resumed = parse_commit("chore(loom): resume\n\nAgent-Id: ratchet\nSession-Id: a1b2c3d4-e5f6-4789-8abc-def012345678\nTask-Status: IMPLEMENTING\nHeartbeat: 2026-04-05T13:30:00Z").unwrap();
        let commits = vec![
            assigned_commit(),
            valid_implementing(),
            blocked,
            resumed,
            valid_completed(),
        ];
        assert_eq!(validate_branch_history(&commits), vec![]);
    }

    #[test]
    fn test_branch_history_first_commit_not_assigned() {
        let errors = validate_branch_history(&[valid_implementing()]);
        assert!(
            errors.iter().any(|e| e.message.contains("ASSIGNED")),
            "expected ASSIGNED error, got: {errors:?}"
        );
    }

    #[test]
    fn test_branch_history_invalid_transition_assigned_to_completed() {
        let commits = vec![assigned_commit(), valid_completed()];
        let errors = validate_branch_history(&commits);
        assert!(
            errors.iter().any(|e| e.message.contains("invalid state transition")),
            "expected transition error, got: {errors:?}"
        );
    }

    #[test]
    fn test_branch_history_commit_after_terminal() {
        let work = parse_commit("feat(loom): extra work\n\nAgent-Id: ratchet\nSession-Id: f8309ae8-ac76-4766-8926-362cdd06d04b\nTask-Status: IMPLEMENTING\nHeartbeat: 2026-04-05T15:00:00Z").unwrap();
        let commits = vec![
            assigned_commit(),
            valid_implementing(),
            valid_completed(),
            work,
        ];
        let errors = validate_branch_history(&commits);
        assert!(
            errors.iter().any(|e| e.message.contains("terminal")),
            "expected post-terminal error, got: {errors:?}"
        );
    }

    // ── Format validator tests ──

    #[test]
    fn test_is_kebab_case() {
        assert!(is_kebab_case("ratchet"));
        assert!(is_kebab_case("bitswell"));
        assert!(is_kebab_case("plugin-scaffold"));
        assert!(is_kebab_case("loom-worker-v2"));
        assert!(is_kebab_case("test-1"));
        assert!(!is_kebab_case(""));
        assert!(!is_kebab_case("Ratchet"));
        assert!(!is_kebab_case("my agent"));
        assert!(!is_kebab_case("-leading"));
        assert!(!is_kebab_case("trailing-"));
        assert!(!is_kebab_case("double--hyphen"));
    }

    #[test]
    fn test_is_uuid_v4() {
        assert!(is_uuid_v4("f8309ae8-ac76-4766-8926-362cdd06d04b"));
        assert!(is_uuid_v4("a1b2c3d4-e5f6-4789-8abc-def012345678"));
        assert!(!is_uuid_v4("not-a-uuid"));
        assert!(!is_uuid_v4("f8309ae8-ac76-3766-8926-362cdd06d04b")); // version 3, not 4
        assert!(!is_uuid_v4("f8309ae8-ac76-4766-0926-362cdd06d04b")); // invalid variant
        assert!(!is_uuid_v4("ratchet-test-1-2026-04-05")); // wrong format
        assert!(!is_uuid_v4(""));
    }

    #[test]
    fn test_is_rfc3339() {
        assert!(is_rfc3339("2026-04-05T12:00:00Z"));
        assert!(is_rfc3339("2026-04-03T14:45:00Z"));
        assert!(is_rfc3339("2026-04-05T12:00:00.123Z"));
        assert!(is_rfc3339("2026-04-05T12:00:00+05:30"));
        assert!(is_rfc3339("2026-04-05T12:00:00-07:00"));
        assert!(!is_rfc3339("2026-04-05 12:00:00Z")); // space instead of T
        assert!(!is_rfc3339("2026-04-05T12:00:00"));   // no timezone
        assert!(!is_rfc3339("not-a-date"));
        assert!(!is_rfc3339(""));
    }
}
