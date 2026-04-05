use std::collections::HashMap;

/// Parsed ASSIGNED commit with all required trailers per schemas.md §5.1.
#[derive(Debug, Clone)]
pub struct AssignedCommit {
    pub agent_id: String,
    pub session_id: String,
    pub assigned_to: String,
    pub assignment: String,
    pub scope: String,
    pub dependencies: String,
    pub budget: String,
}

/// Extract the trailer block from a commit message.
///
/// Git trailers appear after the last blank line in the commit message body.
fn trailer_block(message: &str) -> &str {
    match message.rfind("\n\n") {
        Some(pos) => &message[pos + 2..],
        None => message,
    }
}

/// Parse all trailers from a commit message into a key-value map.
pub fn parse_trailers(message: &str) -> HashMap<String, String> {
    let block = trailer_block(message);
    let mut trailers = HashMap::new();
    for line in block.lines() {
        let trimmed = line.trim();
        if let Some((key, value)) = trimmed.split_once(": ") {
            trailers.insert(key.to_string(), value.to_string());
        }
    }
    trailers
}

/// Check if a commit message contains `Task-Status: ASSIGNED` in its trailers.
pub fn is_assigned_commit(message: &str) -> bool {
    let trailers = parse_trailers(message);
    trailers.get("Task-Status").map_or(false, |v| v == "ASSIGNED")
}

/// Parse an ASSIGNED commit's trailers into a structured type.
/// Returns None if required trailers are missing or Task-Status is not ASSIGNED.
pub fn parse_assignment(message: &str) -> Option<AssignedCommit> {
    let trailers = parse_trailers(message);

    if trailers.get("Task-Status")? != "ASSIGNED" {
        return None;
    }

    Some(AssignedCommit {
        agent_id: trailers.get("Agent-Id")?.clone(),
        session_id: trailers.get("Session-Id")?.clone(),
        assigned_to: trailers.get("Assigned-To")?.clone(),
        assignment: trailers.get("Assignment")?.clone(),
        scope: trailers.get("Scope")?.clone(),
        dependencies: trailers.get("Dependencies")?.clone(),
        budget: trailers.get("Budget")?.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FULL_ASSIGNED: &str = "task(ratchet): implement webhook\n\n\
        Full description here.\n\n\
        Agent-Id: bitswell\n\
        Session-Id: abc-123\n\
        Task-Status: ASSIGNED\n\
        Assigned-To: ratchet\n\
        Assignment: 3-dispatch-trigger\n\
        Scope: .\n\
        Dependencies: none\n\
        Budget: 100000";

    #[test]
    fn detects_assigned() {
        assert!(is_assigned_commit(FULL_ASSIGNED));
    }

    #[test]
    fn parses_full_assignment() {
        let a = parse_assignment(FULL_ASSIGNED).unwrap();
        assert_eq!(a.agent_id, "bitswell");
        assert_eq!(a.assigned_to, "ratchet");
        assert_eq!(a.assignment, "3-dispatch-trigger");
        assert_eq!(a.scope, ".");
        assert_eq!(a.dependencies, "none");
        assert_eq!(a.budget, "100000");
    }

    #[test]
    fn returns_none_for_incomplete_assignment() {
        let msg = "task(ratchet): test\n\nAgent-Id: bitswell\nTask-Status: ASSIGNED";
        assert!(parse_assignment(msg).is_none());
    }

    #[test]
    fn ignores_implementing() {
        let msg = "feat(webhook): add server\n\nAgent-Id: ratchet\nTask-Status: IMPLEMENTING";
        assert!(!is_assigned_commit(msg));
    }

    #[test]
    fn no_trailers() {
        let msg = "fix: typo";
        assert!(!is_assigned_commit(msg));
    }

    #[test]
    fn assigned_in_body_not_trailers() {
        let msg = "task: test\n\nTask-Status: ASSIGNED was mentioned here.\n\nAgent-Id: bitswell\nTask-Status: IMPLEMENTING";
        assert!(!is_assigned_commit(msg));
    }

    #[test]
    fn parse_trailers_extracts_all() {
        let trailers = parse_trailers(FULL_ASSIGNED);
        assert_eq!(trailers.len(), 8);
        assert_eq!(trailers["Agent-Id"], "bitswell");
        assert_eq!(trailers["Budget"], "100000");
    }
}
