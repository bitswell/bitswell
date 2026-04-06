//! LOOM commit message parser and validator.
//!
//! Implements the LOOM protocol v2 schemas defined in
//! `loom/skills/orchestrate/references/schemas.md`.

// ── Types ─────────────────────────────────────────────────────────────────────

/// Parsed commit message header.
#[derive(Debug, Clone, PartialEq)]
pub struct CommitHeader {
    pub commit_type: String,
    pub scope: Option<String>,
    pub subject: String,
}

/// A fully parsed LOOM commit message.
#[derive(Debug, Clone)]
pub struct CommitMessage {
    pub header: CommitHeader,
    pub body: Option<String>,
    /// Ordered trailer pairs; repeated keys are preserved.
    pub trailers: Vec<(String, String)>,
}

impl CommitMessage {
    /// First value of a trailer (case-insensitive key match).
    pub fn trailer(&self, key: &str) -> Option<&str> {
        self.trailers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.as_str())
    }

    /// All values of a trailer (case-insensitive key match).
    pub fn trailer_all(&self, key: &str) -> Vec<&str> {
        self.trailers
            .iter()
            .filter(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.as_str())
            .collect()
    }

    /// Parsed Task-Status, if present.
    pub fn task_status(&self) -> Option<TaskStatus> {
        self.trailer("Task-Status")?.parse().ok()
    }
}

/// LOOM task lifecycle states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Assigned,
    Implementing,
    Completed,
    Blocked,
    Failed,
}

impl std::str::FromStr for TaskStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "ASSIGNED" => Ok(Self::Assigned),
            "IMPLEMENTING" => Ok(Self::Implementing),
            "COMPLETED" => Ok(Self::Completed),
            "BLOCKED" => Ok(Self::Blocked),
            "FAILED" => Ok(Self::Failed),
            other => Err(format!("invalid Task-Status value: {other:?}")),
        }
    }
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Assigned => "ASSIGNED",
            Self::Implementing => "IMPLEMENTING",
            Self::Completed => "COMPLETED",
            Self::Blocked => "BLOCKED",
            Self::Failed => "FAILED",
        };
        write!(f, "{s}")
    }
}

/// A single validation error.
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
        write!(f, "{}", self.message)
    }
}

// ── Parsing ───────────────────────────────────────────────────────────────────

/// Parse a raw commit message string into a [`CommitMessage`].
pub fn parse_commit_message(msg: &str) -> Result<CommitMessage, String> {
    let mut lines = msg.lines();
    let header_line = lines.next().ok_or("empty commit message")?;
    let header = parse_header(header_line)?;
    let rest: Vec<&str> = lines.collect();
    let (body, trailers) = split_body_and_trailers(&rest);
    Ok(CommitMessage {
        header,
        body,
        trailers,
    })
}

fn parse_header(line: &str) -> Result<CommitHeader, String> {
    let colon_pos = line
        .find(": ")
        .ok_or_else(|| format!("no ': ' in header: {line:?}"))?;
    let type_scope = &line[..colon_pos];
    let subject = line[colon_pos + 2..].trim().to_string();
    if subject.is_empty() {
        return Err("empty commit subject".to_string());
    }
    if let Some(open) = type_scope.find('(') {
        let close = type_scope
            .rfind(')')
            .ok_or("unclosed '(' in commit type/scope")?;
        let commit_type = type_scope[..open].trim().to_string();
        let scope = type_scope[open + 1..close].trim().to_string();
        if commit_type.is_empty() {
            return Err("empty commit type".to_string());
        }
        Ok(CommitHeader {
            commit_type,
            scope: if scope.is_empty() { None } else { Some(scope) },
            subject,
        })
    } else {
        let commit_type = type_scope.trim().to_string();
        if commit_type.is_empty() {
            return Err("empty commit type".to_string());
        }
        Ok(CommitHeader {
            commit_type,
            scope: None,
            subject,
        })
    }
}

/// True if `line` is a git trailer: `Token: value` with no leading whitespace.
fn is_trailer_line(line: &str) -> bool {
    if line.is_empty() || line.starts_with(|c: char| c.is_whitespace()) {
        return false;
    }
    if let Some(colon) = line.find(": ") {
        let key = &line[..colon];
        !key.is_empty()
            && key
                .chars()
                .all(|c: char| c.is_alphanumeric() || c == '-')
            && !line[colon + 2..].trim().is_empty()
    } else {
        false
    }
}

/// Split lines after the header into (body, trailers).
///
/// Trailers are the last block of `Key: Value` lines preceded by a blank line.
fn split_body_and_trailers(lines: &[&str]) -> (Option<String>, Vec<(String, String)>) {
    if let Some(blank) = lines.iter().rposition(|l| l.is_empty()) {
        let tail = &lines[blank + 1..];
        if !tail.is_empty() && tail.iter().all(|l| l.is_empty() || is_trailer_line(l)) {
            let trailers: Vec<(String, String)> = tail
                .iter()
                .filter(|l| !l.is_empty())
                .map(|l| {
                    let colon = l.find(": ").unwrap();
                    (l[..colon].to_string(), l[colon + 2..].trim().to_string())
                })
                .collect();
            if !trailers.is_empty() {
                let body = lines[..blank].join("\n").trim().to_string();
                return (
                    if body.is_empty() { None } else { Some(body) },
                    trailers,
                );
            }
        }
    }
    let body = lines.join("\n").trim().to_string();
    (if body.is_empty() { None } else { Some(body) }, vec![])
}

// ── Format validators ─────────────────────────────────────────────────────────

/// True if `s` matches `[a-z0-9]+(-[a-z0-9]+)*` (kebab-case).
fn is_kebab_case(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    s.split('-').all(|seg| {
        !seg.is_empty()
            && seg
                .chars()
                .all(|c: char| c.is_ascii_lowercase() || c.is_ascii_digit())
    })
}

/// True if `s` is a valid UUID v4: `xxxxxxxx-xxxx-4xxx-[89ab]xxx-xxxxxxxxxxxx`.
fn is_valid_uuid_v4(s: &str) -> bool {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 5 {
        return false;
    }
    for (part, &expected) in parts.iter().zip(&[8usize, 4, 4, 4, 12]) {
        if part.len() != expected || !part.chars().all(|c: char| c.is_ascii_hexdigit()) {
            return false;
        }
    }
    // Version nibble must be '4'.
    if !parts[2].starts_with('4') {
        return false;
    }
    // Variant nibble must be 8, 9, a, or b.
    matches!(
        parts[3].chars().next().unwrap(),
        '8' | '9' | 'a' | 'b' | 'A' | 'B'
    )
}

// ── Public validation API ─────────────────────────────────────────────────────

/// Validate a single commit message against LOOM per-commit rules (schema §7.1).
///
/// Returns an empty `Vec` if the commit is valid.
pub fn validate_commit(msg: &str) -> Vec<ValidationError> {
    let commit = match parse_commit_message(msg) {
        Ok(c) => c,
        Err(e) => return vec![ValidationError::new(format!("parse error: {e}"))],
    };

    let mut errors = vec![];

    // Rules 1 & 2: Agent-Id required, must be kebab-case.
    match commit.trailer("Agent-Id") {
        None => errors.push(ValidationError::new("missing required trailer: Agent-Id")),
        Some(id) if !is_kebab_case(id) => errors.push(ValidationError::new(format!(
            "Agent-Id must be kebab-case: {id:?}"
        ))),
        _ => {}
    }

    // Rules 1 & 3: Session-Id required, must be UUID v4.
    match commit.trailer("Session-Id") {
        None => errors.push(ValidationError::new("missing required trailer: Session-Id")),
        Some(sid) if !is_valid_uuid_v4(sid) => errors.push(ValidationError::new(format!(
            "Session-Id must be UUID v4: {sid:?}"
        ))),
        _ => {}
    }

    // Rule 4: Task-Status must be a valid value if present.
    if let Some(status_str) = commit.trailer("Task-Status") {
        let status: Option<TaskStatus> = match status_str.parse() {
            Ok(s) => Some(s),
            Err(e) => {
                errors.push(ValidationError::new(e));
                None
            }
        };

        if let Some(status) = status {
            match status {
                // Rule 5: ASSIGNED requires assignment trailers.
                TaskStatus::Assigned => {
                    for key in ["Assigned-To", "Assignment", "Scope", "Dependencies", "Budget"] {
                        if commit.trailer(key).is_none() {
                            errors.push(ValidationError::new(format!(
                                "ASSIGNED commit missing required trailer: {key}"
                            )));
                        }
                    }
                    if let Some(budget) = commit.trailer("Budget") {
                        if budget.parse::<u64>().is_err() {
                            errors.push(ValidationError::new(format!(
                                "Budget must be a positive integer: {budget:?}"
                            )));
                        }
                    }
                }

                // Rule 6: COMPLETED requires Files-Changed and at least one Key-Finding.
                TaskStatus::Completed => {
                    match commit.trailer("Files-Changed") {
                        None => errors.push(ValidationError::new(
                            "COMPLETED commit missing required trailer: Files-Changed",
                        )),
                        Some(fc) if fc.parse::<u64>().is_err() => {
                            errors.push(ValidationError::new(format!(
                                "Files-Changed must be a non-negative integer: {fc:?}"
                            )));
                        }
                        _ => {}
                    }
                    if commit.trailer("Key-Finding").is_none() {
                        errors.push(ValidationError::new(
                            "COMPLETED commit missing required trailer: Key-Finding",
                        ));
                    }
                    if commit.trailer("Heartbeat").is_none() {
                        errors.push(ValidationError::new(
                            "COMPLETED commit missing required trailer: Heartbeat",
                        ));
                    }
                }

                // Rule 7: BLOCKED requires Blocked-Reason.
                TaskStatus::Blocked => {
                    if commit.trailer("Blocked-Reason").is_none() {
                        errors.push(ValidationError::new(
                            "BLOCKED commit missing required trailer: Blocked-Reason",
                        ));
                    }
                    if commit.trailer("Heartbeat").is_none() {
                        errors.push(ValidationError::new(
                            "BLOCKED commit missing required trailer: Heartbeat",
                        ));
                    }
                }

                // Rule 8: FAILED requires Error-Category and Error-Retryable.
                TaskStatus::Failed => {
                    for key in ["Error-Category", "Error-Retryable"] {
                        if commit.trailer(key).is_none() {
                            errors.push(ValidationError::new(format!(
                                "FAILED commit missing required trailer: {key}"
                            )));
                        }
                    }
                    if let Some(cat) = commit.trailer("Error-Category") {
                        const VALID_CATS: &[&str] = &[
                            "task_unclear",
                            "blocked",
                            "resource_limit",
                            "conflict",
                            "internal",
                        ];
                        if !VALID_CATS.contains(&cat) {
                            errors.push(ValidationError::new(format!(
                                "invalid Error-Category: {cat:?} (must be one of: {})",
                                VALID_CATS.join(", ")
                            )));
                        }
                    }
                    if let Some(ret) = commit.trailer("Error-Retryable") {
                        if ret != "true" && ret != "false" {
                            errors.push(ValidationError::new(format!(
                                "Error-Retryable must be 'true' or 'false': {ret:?}"
                            )));
                        }
                    }
                }

                // IMPLEMENTING requires Heartbeat.
                TaskStatus::Implementing => {
                    if commit.trailer("Heartbeat").is_none() {
                        errors.push(ValidationError::new(
                            "IMPLEMENTING commit missing required trailer: Heartbeat",
                        ));
                    }
                }
            }
        }
    }

    errors
}

/// Validate a branch name against LOOM naming convention (schema §2).
///
/// Returns an empty `Vec` if the name is valid.
pub fn validate_branch_name(name: &str) -> Vec<ValidationError> {
    let mut errors = vec![];

    if name.len() > 63 {
        errors.push(ValidationError::new(format!(
            "branch name exceeds 63 characters ({} chars): {name:?}",
            name.len()
        )));
    }

    let Some(rest) = name.strip_prefix("loom/") else {
        errors.push(ValidationError::new("branch name must start with 'loom/'"));
        return errors;
    };

    let Some(hyphen) = rest.find('-') else {
        errors.push(ValidationError::new(
            "branch name must have format loom/<agent>-<slug>",
        ));
        return errors;
    };

    let agent = &rest[..hyphen];
    let slug = &rest[hyphen + 1..];

    if !is_kebab_case(agent) {
        errors.push(ValidationError::new(format!(
            "agent part of branch name must be kebab-case: {agent:?}"
        )));
    }
    if slug.is_empty() {
        errors.push(ValidationError::new("slug part of branch name is empty"));
    } else if !is_kebab_case(slug) {
        errors.push(ValidationError::new(format!(
            "slug part of branch name must be kebab-case: {slug:?}"
        )));
    }

    errors
}

/// Validate a branch's commit sequence against LOOM branch-level rules (schema §7.2).
///
/// `messages` is an ordered slice of raw commit messages, **oldest first**.
/// Returns an empty `Vec` if the sequence is valid.
pub fn validate_branch_sequence(messages: &[&str]) -> Vec<ValidationError> {
    let mut errors = vec![];

    if messages.is_empty() {
        errors.push(ValidationError::new("branch has no commits"));
        return errors;
    }

    // Rule 9: first commit must be ASSIGNED.
    let first = match parse_commit_message(messages[0]) {
        Ok(c) => c,
        Err(e) => {
            errors.push(ValidationError::new(format!("first commit parse error: {e}")));
            return errors;
        }
    };
    if first.task_status() != Some(TaskStatus::Assigned) {
        errors.push(ValidationError::new(format!(
            "first commit must have Task-Status: ASSIGNED, got: {:?}",
            first.trailer("Task-Status")
        )));
    }

    let mut prev_status: Option<TaskStatus> = first.task_status();
    let mut terminal_seen = false;
    let mut terminal_count: usize = 0;

    for (i, msg) in messages[1..].iter().enumerate() {
        let idx = i + 2; // human-readable commit number (1-based, first = 1)
        let commit = match parse_commit_message(msg) {
            Ok(c) => c,
            Err(e) => {
                errors.push(ValidationError::new(format!("commit {idx} parse error: {e}")));
                continue;
            }
        };

        let status = commit.task_status();

        // Rule 12: no Task-Status after a terminal commit.
        if terminal_seen {
            if status.is_some() {
                errors.push(ValidationError::new(format!(
                    "commit {idx}: Task-Status present after terminal state"
                )));
            }
            continue;
        }

        // Rules 11 & 13: track terminal commits.
        if matches!(status, Some(TaskStatus::Completed) | Some(TaskStatus::Failed)) {
            terminal_count += 1;
            terminal_seen = true;
            if terminal_count > 1 {
                errors.push(ValidationError::new(format!(
                    "commit {idx}: branch already has a terminal commit"
                )));
            }
        }

        // Validate state machine transitions.
        if let Some(curr) = status {
            if let Some(err) = transition_error(prev_status, curr, idx) {
                errors.push(err);
            } else {
                prev_status = Some(curr);
            }
        }
    }

    errors
}

/// Returns an error if the transition `from -> to` is invalid, `None` if valid.
fn transition_error(
    from: Option<TaskStatus>,
    to: TaskStatus,
    idx: usize,
) -> Option<ValidationError> {
    match (from, to) {
        // Valid transitions.
        (Some(TaskStatus::Assigned), TaskStatus::Implementing) => None,
        (Some(TaskStatus::Implementing), TaskStatus::Completed) => None,
        (Some(TaskStatus::Implementing), TaskStatus::Blocked) => None,
        (Some(TaskStatus::Implementing), TaskStatus::Failed) => None,
        (Some(TaskStatus::Blocked), TaskStatus::Implementing) => None,
        (Some(TaskStatus::Blocked), TaskStatus::Failed) => None,
        // ASSIGNED may only appear once (as the very first commit).
        (_, TaskStatus::Assigned) => Some(ValidationError::new(format!(
            "commit {idx}: ASSIGNED may only appear as the first commit"
        ))),
        // BLOCKED -> COMPLETED is forbidden; must resume IMPLEMENTING first.
        (Some(TaskStatus::Blocked), TaskStatus::Completed) => Some(ValidationError::new(format!(
            "commit {idx}: invalid transition BLOCKED -> COMPLETED; must resume IMPLEMENTING first"
        ))),
        (from, to) => Some(ValidationError::new(format!(
            "commit {idx}: invalid transition {} -> {to}",
            from.map(|s| s.to_string())
                .unwrap_or_else(|| "none".to_string())
        ))),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // A valid UUID v4 used across tests (from LOOM examples).
    const UUID: &str = "f8309ae8-ac76-4766-8926-362cdd06d04b";

    // ── parse_commit_message ─────────────────────────────────────────────────

    #[test]
    fn parse_header_with_scope() {
        let msg = format!(
            "chore(loom): begin implementing trailer parser\n\nAgent-Id: ratchet\nSession-Id: {UUID}"
        );
        let commit = parse_commit_message(&msg).unwrap();
        assert_eq!(commit.header.commit_type, "chore");
        assert_eq!(commit.header.scope, Some("loom".to_string()));
        assert_eq!(commit.header.subject, "begin implementing trailer parser");
        assert_eq!(commit.trailer("Agent-Id"), Some("ratchet"));
        assert_eq!(commit.trailer("Session-Id"), Some(UUID));
    }

    #[test]
    fn parse_header_without_scope() {
        let msg = format!("feat: add feature\n\nAgent-Id: moss\nSession-Id: {UUID}");
        let commit = parse_commit_message(&msg).unwrap();
        assert_eq!(commit.header.commit_type, "feat");
        assert_eq!(commit.header.scope, None);
        assert_eq!(commit.header.subject, "add feature");
    }

    #[test]
    fn parse_body_and_trailers() {
        let msg = format!(
            "feat(loom): add feature\n\nThis explains the why.\nMore context.\n\nAgent-Id: ratchet\nSession-Id: {UUID}"
        );
        let commit = parse_commit_message(&msg).unwrap();
        assert_eq!(
            commit.body.as_deref(),
            Some("This explains the why.\nMore context.")
        );
        assert_eq!(commit.trailer("Agent-Id"), Some("ratchet"));
    }

    #[test]
    fn parse_no_body_trailers_only() {
        let msg = format!("chore(loom): checkpoint\n\nAgent-Id: ratchet\nSession-Id: {UUID}");
        let commit = parse_commit_message(&msg).unwrap();
        assert!(commit.body.is_none());
        assert_eq!(commit.trailer("Agent-Id"), Some("ratchet"));
    }

    #[test]
    fn parse_repeated_trailer_key() {
        let msg = format!(
            "feat(loom): done\n\nAgent-Id: ratchet\nSession-Id: {UUID}\nKey-Finding: first finding\nKey-Finding: second finding"
        );
        let commit = parse_commit_message(&msg).unwrap();
        let findings = commit.trailer_all("Key-Finding");
        assert_eq!(findings, vec!["first finding", "second finding"]);
    }

    #[test]
    fn parse_error_on_empty_message() {
        assert!(parse_commit_message("").is_err());
    }

    #[test]
    fn parse_error_on_no_colon_space() {
        assert!(parse_commit_message("notaheader").is_err());
    }

    // ── is_kebab_case ────────────────────────────────────────────────────────

    #[test]
    fn kebab_case_valid() {
        assert!(is_kebab_case("ratchet"));
        assert!(is_kebab_case("plugin-scaffold"));
        assert!(is_kebab_case("loom-worker"));
        assert!(is_kebab_case("test1"));
        assert!(is_kebab_case("a-b-c"));
    }

    #[test]
    fn kebab_case_invalid() {
        assert!(!is_kebab_case(""));
        assert!(!is_kebab_case("-leading"));
        assert!(!is_kebab_case("trailing-"));
        assert!(!is_kebab_case("double--dash"));
        assert!(!is_kebab_case("Upper"));
        assert!(!is_kebab_case("has space"));
    }

    // ── is_valid_uuid_v4 ─────────────────────────────────────────────────────

    #[test]
    fn uuid_v4_valid() {
        assert!(is_valid_uuid_v4("f8309ae8-ac76-4766-8926-362cdd06d04b"));
        assert!(is_valid_uuid_v4("550e8400-e29b-41d4-a716-446655440000"));
        assert!(is_valid_uuid_v4("6ba7b810-9dad-41d1-80b4-00c04fd430c8"));
    }

    #[test]
    fn uuid_v4_invalid_version() {
        // Version nibble is '3', not '4'.
        assert!(!is_valid_uuid_v4("f8309ae8-ac76-3766-8926-362cdd06d04b"));
    }

    #[test]
    fn uuid_v4_invalid_variant() {
        // Variant nibble is 'c', not [89ab].
        assert!(!is_valid_uuid_v4("f8309ae8-ac76-4766-c926-362cdd06d04b"));
    }

    #[test]
    fn uuid_v4_invalid_format() {
        assert!(!is_valid_uuid_v4("not-a-uuid"));
        assert!(!is_valid_uuid_v4("abc123"));
        assert!(!is_valid_uuid_v4(""));
        assert!(!is_valid_uuid_v4("f8309ae8-ac76-4766-8926")); // too short
    }

    // ── validate_commit ──────────────────────────────────────────────────────

    fn assigned_commit() -> String {
        format!(
            "task(ratchet): scaffold plugin\n\nCreate the scaffold.\n\nAgent-Id: bitswell\nSession-Id: {UUID}\nTask-Status: ASSIGNED\nAssigned-To: ratchet\nAssignment: plugin-scaffold\nScope: loom/**\nDependencies: none\nBudget: 100000"
        )
    }

    fn implementing_commit() -> String {
        format!(
            "chore(loom): begin implementing\n\nAgent-Id: ratchet\nSession-Id: {UUID}\nTask-Status: IMPLEMENTING\nHeartbeat: 2026-04-05T10:00:00Z"
        )
    }

    fn completed_commit() -> String {
        format!(
            "feat(loom): implement trailer parser\n\nBuilt the parser.\n\nAgent-Id: ratchet\nSession-Id: {UUID}\nTask-Status: COMPLETED\nFiles-Changed: 3\nKey-Finding: parser handles all LOOM trailer formats\nHeartbeat: 2026-04-05T12:00:00Z"
        )
    }

    #[test]
    fn valid_assigned_commit() {
        let errors = validate_commit(&assigned_commit());
        assert!(errors.is_empty(), "unexpected errors: {errors:?}");
    }

    #[test]
    fn valid_implementing_commit() {
        let errors = validate_commit(&implementing_commit());
        assert!(errors.is_empty(), "unexpected errors: {errors:?}");
    }

    #[test]
    fn valid_completed_commit() {
        let errors = validate_commit(&completed_commit());
        assert!(errors.is_empty(), "unexpected errors: {errors:?}");
    }

    #[test]
    fn valid_work_commit_no_status() {
        let msg = format!(
            "refactor(loom): extract helper\n\nAgent-Id: ratchet\nSession-Id: {UUID}\nHeartbeat: 2026-04-05T11:00:00Z"
        );
        let errors = validate_commit(&msg);
        assert!(errors.is_empty(), "unexpected errors: {errors:?}");
    }

    #[test]
    fn valid_blocked_commit() {
        let msg = format!(
            "chore(loom): blocked -- missing dep\n\nAgent-Id: ratchet\nSession-Id: {UUID}\nTask-Status: BLOCKED\nBlocked-Reason: dependency not yet merged\nHeartbeat: 2026-04-05T11:00:00Z"
        );
        let errors = validate_commit(&msg);
        assert!(errors.is_empty(), "unexpected errors: {errors:?}");
    }

    #[test]
    fn valid_failed_commit() {
        let msg = format!(
            "chore(loom): failed -- unrecoverable\n\nAgent-Id: ratchet\nSession-Id: {UUID}\nTask-Status: FAILED\nError-Category: internal\nError-Retryable: false"
        );
        let errors = validate_commit(&msg);
        assert!(errors.is_empty(), "unexpected errors: {errors:?}");
    }

    #[test]
    fn missing_agent_id() {
        let msg = format!("feat(loom): thing\n\nSession-Id: {UUID}");
        let errors = validate_commit(&msg);
        assert!(
            errors.iter().any(|e| e.message.contains("Agent-Id")),
            "expected Agent-Id error, got: {errors:?}"
        );
    }

    #[test]
    fn missing_session_id() {
        let msg = "feat(loom): thing\n\nAgent-Id: ratchet";
        let errors = validate_commit(msg);
        assert!(
            errors.iter().any(|e| e.message.contains("Session-Id")),
            "expected Session-Id error, got: {errors:?}"
        );
    }

    #[test]
    fn invalid_agent_id_not_kebab() {
        let msg = format!("feat(loom): thing\n\nAgent-Id: MyAgent\nSession-Id: {UUID}");
        let errors = validate_commit(&msg);
        assert!(
            errors.iter().any(|e| e.message.contains("Agent-Id")),
            "expected Agent-Id error, got: {errors:?}"
        );
    }

    #[test]
    fn invalid_session_id_not_uuid() {
        let msg = "feat(loom): thing\n\nAgent-Id: ratchet\nSession-Id: not-a-uuid";
        let errors = validate_commit(msg);
        assert!(
            errors.iter().any(|e| e.message.contains("Session-Id")),
            "expected Session-Id error, got: {errors:?}"
        );
    }

    #[test]
    fn invalid_task_status_value() {
        let msg = format!(
            "feat(loom): thing\n\nAgent-Id: ratchet\nSession-Id: {UUID}\nTask-Status: UNKNOWN"
        );
        let errors = validate_commit(&msg);
        assert!(
            errors.iter().any(|e| e.message.contains("Task-Status")),
            "expected Task-Status error, got: {errors:?}"
        );
    }

    #[test]
    fn assigned_missing_required_trailers() {
        let msg = format!(
            "task(ratchet): do thing\n\nAgent-Id: bitswell\nSession-Id: {UUID}\nTask-Status: ASSIGNED\nAssigned-To: ratchet"
            // missing: Assignment, Scope, Dependencies, Budget
        );
        let errors = validate_commit(&msg);
        assert!(
            errors.iter().any(|e| e.message.contains("Assignment")),
            "expected Assignment error, got: {errors:?}"
        );
        assert!(
            errors.iter().any(|e| e.message.contains("Scope")),
            "expected Scope error, got: {errors:?}"
        );
        assert!(
            errors.iter().any(|e| e.message.contains("Budget")),
            "expected Budget error, got: {errors:?}"
        );
    }

    #[test]
    fn completed_missing_files_changed() {
        let msg = format!(
            "feat(loom): done\n\nAgent-Id: ratchet\nSession-Id: {UUID}\nTask-Status: COMPLETED\nKey-Finding: found something\nHeartbeat: 2026-04-05T12:00:00Z"
        );
        let errors = validate_commit(&msg);
        assert!(
            errors.iter().any(|e| e.message.contains("Files-Changed")),
            "expected Files-Changed error, got: {errors:?}"
        );
    }

    #[test]
    fn completed_missing_key_finding() {
        let msg = format!(
            "feat(loom): done\n\nAgent-Id: ratchet\nSession-Id: {UUID}\nTask-Status: COMPLETED\nFiles-Changed: 2\nHeartbeat: 2026-04-05T12:00:00Z"
        );
        let errors = validate_commit(&msg);
        assert!(
            errors.iter().any(|e| e.message.contains("Key-Finding")),
            "expected Key-Finding error, got: {errors:?}"
        );
    }

    #[test]
    fn implementing_missing_heartbeat() {
        let msg = format!(
            "chore(loom): begin\n\nAgent-Id: ratchet\nSession-Id: {UUID}\nTask-Status: IMPLEMENTING"
        );
        let errors = validate_commit(&msg);
        assert!(
            errors.iter().any(|e| e.message.contains("Heartbeat")),
            "expected Heartbeat error, got: {errors:?}"
        );
    }

    #[test]
    fn failed_invalid_error_category() {
        let msg = format!(
            "chore(loom): failed\n\nAgent-Id: ratchet\nSession-Id: {UUID}\nTask-Status: FAILED\nError-Category: oops\nError-Retryable: false"
        );
        let errors = validate_commit(&msg);
        assert!(
            errors.iter().any(|e| e.message.contains("Error-Category")),
            "expected Error-Category error, got: {errors:?}"
        );
    }

    #[test]
    fn failed_invalid_error_retryable() {
        let msg = format!(
            "chore(loom): failed\n\nAgent-Id: ratchet\nSession-Id: {UUID}\nTask-Status: FAILED\nError-Category: internal\nError-Retryable: yes"
        );
        let errors = validate_commit(&msg);
        assert!(
            errors.iter().any(|e| e.message.contains("Error-Retryable")),
            "expected Error-Retryable error, got: {errors:?}"
        );
    }

    // ── validate_branch_name ─────────────────────────────────────────────────

    #[test]
    fn valid_branch_names() {
        assert!(validate_branch_name("loom/ratchet-plugin-scaffold").is_empty());
        assert!(validate_branch_name("loom/moss-migrate-identities").is_empty());
        assert!(validate_branch_name("loom/bitswell-test-1").is_empty());
    }

    #[test]
    fn branch_name_missing_prefix() {
        let errors = validate_branch_name("ratchet-plugin-scaffold");
        assert!(
            errors.iter().any(|e| e.message.contains("loom/")),
            "expected loom/ prefix error, got: {errors:?}"
        );
    }

    #[test]
    fn branch_name_missing_slug() {
        let errors = validate_branch_name("loom/ratchet");
        assert!(!errors.is_empty(), "expected error for missing slug");
    }

    #[test]
    fn branch_name_too_long() {
        let long = format!("loom/ratchet-{}", "x".repeat(60));
        let errors = validate_branch_name(&long);
        assert!(
            errors.iter().any(|e| e.message.contains("63")),
            "expected length error, got: {errors:?}"
        );
    }

    #[test]
    fn branch_name_uppercase_invalid() {
        let errors = validate_branch_name("loom/Ratchet-scaffold");
        assert!(!errors.is_empty(), "expected error for uppercase agent name");
    }

    // ── validate_branch_sequence ─────────────────────────────────────────────

    #[test]
    fn valid_full_sequence() {
        let messages = vec![
            assigned_commit(),
            implementing_commit(),
            format!(
                "refactor(loom): extract helper\n\nAgent-Id: ratchet\nSession-Id: {UUID}\nHeartbeat: 2026-04-05T11:30:00Z"
            ),
            completed_commit(),
        ];
        let refs: Vec<&str> = messages.iter().map(|s| s.as_str()).collect();
        let errors = validate_branch_sequence(&refs);
        assert!(errors.is_empty(), "unexpected errors: {errors:?}");
    }

    #[test]
    fn valid_blocked_then_resume() {
        let blocked = format!(
            "chore(loom): blocked\n\nAgent-Id: ratchet\nSession-Id: {UUID}\nTask-Status: BLOCKED\nBlocked-Reason: dep missing\nHeartbeat: 2026-04-05T11:00:00Z"
        );
        let resume = format!(
            "chore(loom): resume after blocker resolved\n\nAgent-Id: ratchet\nSession-Id: {UUID}\nTask-Status: IMPLEMENTING\nHeartbeat: 2026-04-05T11:30:00Z"
        );
        let messages = vec![
            assigned_commit(),
            implementing_commit(),
            blocked,
            resume,
            completed_commit(),
        ];
        let refs: Vec<&str> = messages.iter().map(|s| s.as_str()).collect();
        let errors = validate_branch_sequence(&refs);
        assert!(errors.is_empty(), "unexpected errors: {errors:?}");
    }

    #[test]
    fn sequence_first_commit_not_assigned() {
        let messages = vec![implementing_commit(), completed_commit()];
        let refs: Vec<&str> = messages.iter().map(|s| s.as_str()).collect();
        let errors = validate_branch_sequence(&refs);
        assert!(
            errors.iter().any(|e| e.message.contains("ASSIGNED")),
            "expected ASSIGNED error, got: {errors:?}"
        );
    }

    #[test]
    fn sequence_invalid_transition_assigned_to_completed() {
        let messages = vec![assigned_commit(), completed_commit()];
        let refs: Vec<&str> = messages.iter().map(|s| s.as_str()).collect();
        let errors = validate_branch_sequence(&refs);
        assert!(!errors.is_empty(), "expected transition error");
    }

    #[test]
    fn sequence_blocked_to_completed_invalid() {
        let blocked = format!(
            "chore(loom): blocked\n\nAgent-Id: ratchet\nSession-Id: {UUID}\nTask-Status: BLOCKED\nBlocked-Reason: dep\nHeartbeat: 2026-04-05T11:00:00Z"
        );
        let messages = vec![assigned_commit(), implementing_commit(), blocked, completed_commit()];
        let refs: Vec<&str> = messages.iter().map(|s| s.as_str()).collect();
        let errors = validate_branch_sequence(&refs);
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("BLOCKED") && e.message.contains("COMPLETED")),
            "expected BLOCKED -> COMPLETED error, got: {errors:?}"
        );
    }

    #[test]
    fn sequence_status_after_terminal() {
        let after_done = format!(
            "chore(loom): post-terminal\n\nAgent-Id: bitswell\nSession-Id: {UUID}\nTask-Status: IMPLEMENTING\nHeartbeat: 2026-04-05T13:00:00Z"
        );
        let messages = vec![
            assigned_commit(),
            implementing_commit(),
            completed_commit(),
            after_done,
        ];
        let refs: Vec<&str> = messages.iter().map(|s| s.as_str()).collect();
        let errors = validate_branch_sequence(&refs);
        assert!(
            errors.iter().any(|e| e.message.contains("terminal")),
            "expected post-terminal error, got: {errors:?}"
        );
    }

    #[test]
    fn sequence_second_assigned_invalid() {
        let second_assigned = format!(
            "task(ratchet): reassign\n\nAgent-Id: bitswell\nSession-Id: {UUID}\nTask-Status: ASSIGNED\nAssigned-To: ratchet\nAssignment: retry\nScope: loom/**\nDependencies: none\nBudget: 50000"
        );
        let messages = vec![assigned_commit(), implementing_commit(), second_assigned];
        let refs: Vec<&str> = messages.iter().map(|s| s.as_str()).collect();
        let errors = validate_branch_sequence(&refs);
        assert!(
            errors.iter().any(|e| e.message.contains("ASSIGNED")),
            "expected duplicate ASSIGNED error, got: {errors:?}"
        );
    }

    #[test]
    fn empty_sequence() {
        let errors = validate_branch_sequence(&[]);
        assert!(!errors.is_empty(), "expected error for empty sequence");
    }
}
