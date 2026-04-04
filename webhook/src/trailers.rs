/// Check if a commit message contains `Task-Status: ASSIGNED` in its trailers.
///
/// Git trailers appear after the last blank line in the commit message body.
/// We look for a line matching `Task-Status: ASSIGNED` (case-sensitive, trimmed).
pub fn is_assigned_commit(message: &str) -> bool {
    // Trailers are in the last paragraph (after the last blank line)
    let trailer_block = match message.rfind("\n\n") {
        Some(pos) => &message[pos + 2..],
        None => message,
    };

    trailer_block.lines().any(|line| {
        let trimmed = line.trim();
        trimmed == "Task-Status: ASSIGNED"
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_assigned() {
        let msg = "task(ratchet): implement webhook\n\nFull description here.\n\nAgent-Id: bitswell\nSession-Id: abc-123\nTask-Status: ASSIGNED\nAssigned-To: ratchet";
        assert!(is_assigned_commit(msg));
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
        // "Task-Status: ASSIGNED" in the body paragraph, not trailers
        let msg = "task: test\n\nTask-Status: ASSIGNED was mentioned here.\n\nAgent-Id: bitswell\nTask-Status: IMPLEMENTING";
        assert!(!is_assigned_commit(msg));
    }
}
