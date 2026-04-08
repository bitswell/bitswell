use std::path::PathBuf;
use std::time::Duration;

use serde::Serialize;
use tokio::process::Command;

use crate::trailers::AssignedCommit;

const MAX_RETRIES: u32 = 1;
const RETRY_DELAY: Duration = Duration::from_secs(2);
const MAX_COMMENT_CHARS: usize = 2000;

/// Structured dispatch payload sent as the prompt to Claude CLI.
#[derive(Debug, Serialize)]
struct PushDispatchPayload {
    event: &'static str,
    commit_sha: String,
    branch: String,
    assigned_to: String,
    assignment: String,
    scope: String,
    dependencies: String,
    budget: String,
    agent_id: String,
    session_id: String,
}

#[derive(Debug, Serialize)]
struct IssueDispatchPayload {
    event: &'static str,
    number: u64,
    title: String,
    author: String,
}

#[derive(Debug, Serialize)]
struct CommentDispatchPayload {
    event: &'static str,
    issue_number: u64,
    author: String,
    body: String,
}

pub struct TriggerClient {
    repo_path: PathBuf,
    claude_cmd: String,
}

impl TriggerClient {
    pub fn new(repo_path: PathBuf) -> Self {
        let claude_cmd = std::env::var("CLAUDE_CMD").unwrap_or_else(|_| "claude".into());
        Self {
            repo_path,
            claude_cmd,
        }
    }

    /// Dispatch a LOOM assignment by spawning Claude CLI.
    pub async fn dispatch_push(
        &self,
        commit_sha: &str,
        branch: &str,
        assignment: &AssignedCommit,
    ) -> Result<(), TriggerError> {
        let payload = PushDispatchPayload {
            event: "push.assigned",
            commit_sha: commit_sha.to_string(),
            branch: branch.to_string(),
            assigned_to: assignment.assigned_to.clone(),
            assignment: assignment.assignment.clone(),
            scope: assignment.scope.clone(),
            dependencies: assignment.dependencies.clone(),
            budget: assignment.budget.clone(),
            agent_id: assignment.agent_id.clone(),
            session_id: assignment.session_id.clone(),
        };
        let json = serde_json::to_string_pretty(&payload)
            .unwrap_or_else(|_| format!("{{\"event\":\"push.assigned\",\"commit_sha\":\"{commit_sha}\"}}"));

        let prompt = format!(
            "You are bitswell. A GitHub push event triggered this session.\n\
             The following ASSIGNED commit was detected on a loom/* branch.\n\
             Read CLAUDE.md and AGENT.md, then dispatch the agent.\n\n\
             Event data:\n```json\n{json}\n```\n\n\
             Run: loom/bin/loom-dispatch --commit {commit_sha}"
        );
        self.run_with_retry(&prompt).await
    }

    /// Notify about a new issue.
    pub async fn dispatch_issue(
        &self,
        number: u64,
        title: &str,
        author: &str,
    ) -> Result<(), TriggerError> {
        let payload = IssueDispatchPayload {
            event: "issues.opened",
            number,
            title: title.to_string(),
            author: author.to_string(),
        };
        let json = serde_json::to_string_pretty(&payload)
            .unwrap_or_else(|_| format!("{{\"event\":\"issues.opened\",\"number\":{number}}}"));

        let prompt = format!(
            "You are bitswell. A new GitHub issue was opened.\n\
             Read CLAUDE.md and AGENT.md, then triage this issue.\n\n\
             Event data:\n```json\n{json}\n```\n\n\
             Use `gh issue view {number}` to read the full issue.\n\
             Decide if it needs immediate action, should become a task, or can wait."
        );
        self.run_with_retry(&prompt).await
    }

    /// Notify about a new issue comment.
    pub async fn dispatch_comment(
        &self,
        issue_number: u64,
        author: &str,
        body: &str,
    ) -> Result<(), TriggerError> {
        let truncated = truncate_utf8(body, MAX_COMMENT_CHARS);
        let payload = CommentDispatchPayload {
            event: "issue_comment.created",
            issue_number,
            author: author.to_string(),
            body: truncated,
        };
        let json = serde_json::to_string_pretty(&payload)
            .unwrap_or_else(|_| format!("{{\"event\":\"issue_comment.created\",\"issue_number\":{issue_number}}}"));

        let prompt = format!(
            "You are bitswell. A new comment was posted on a GitHub issue/PR.\n\
             Read CLAUDE.md and AGENT.md, then decide if action is needed.\n\n\
             Event data:\n```json\n{json}\n```\n\n\
             Use `gh issue view {issue_number}` or `gh pr view {issue_number}` to read full context."
        );
        self.run_with_retry(&prompt).await
    }

    async fn run_with_retry(&self, prompt: &str) -> Result<(), TriggerError> {
        let mut last_err = None;
        for attempt in 0..=MAX_RETRIES {
            if attempt > 0 {
                tracing::info!(attempt, "retrying claude dispatch");
                tokio::time::sleep(RETRY_DELAY * attempt).await;
            }
            match self.run_claude(prompt).await {
                Ok(()) => return Ok(()),
                Err(e) => {
                    tracing::warn!(attempt, error = %e, "dispatch error");
                    last_err = Some(e);
                }
            }
        }
        Err(last_err.expect("retry loop should have returned an error"))
    }

    async fn run_claude(&self, prompt: &str) -> Result<(), TriggerError> {
        tracing::info!(cmd = %self.claude_cmd, repo = %self.repo_path.display(), "spawning claude session");

        let output = Command::new(&self.claude_cmd)
            .arg("--print")
            .arg("--max-turns")
            .arg("25")
            .arg("-p")
            .arg(prompt)
            .current_dir(&self.repo_path)
            .output()
            .await
            .map_err(|e| TriggerError::Spawn(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(TriggerError::Exit {
                code: output.status.code(),
                stderr: stderr.into_owned(),
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        tracing::info!(
            stdout_len = stdout.len(),
            "claude session completed"
        );
        Ok(())
    }
}

/// Truncate a string to at most `max_chars` characters (UTF-8 safe).
fn truncate_utf8(s: &str, max_chars: usize) -> String {
    let mut char_iter = s.char_indices();
    let mut byte_end = s.len();
    let mut count = 0;
    for (idx, _) in &mut char_iter {
        if count >= max_chars {
            byte_end = idx;
            break;
        }
        count += 1;
    }
    if count < max_chars {
        return s.to_string();
    }
    format!("{}... (truncated)", &s[..byte_end])
}

#[derive(Debug)]
pub enum TriggerError {
    Spawn(String),
    Exit { code: Option<i32>, stderr: String },
}

impl std::fmt::Display for TriggerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Spawn(e) => write!(f, "failed to spawn claude: {e}"),
            Self::Exit { code, stderr } => {
                write!(f, "claude exited with code {:?}: {}", code, stderr)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_ascii() {
        let s = "a".repeat(3000);
        let t = truncate_utf8(&s, 2000);
        assert!(t.ends_with("... (truncated)"));
        assert_eq!(t.len(), 2000 + "... (truncated)".len());
    }

    #[test]
    fn truncate_utf8_safe() {
        let s: String = std::iter::repeat('🦀').take(501).collect();
        let t = truncate_utf8(&s, 500);
        assert!(t.is_char_boundary(t.len()));
        assert!(t.ends_with("... (truncated)"));
    }

    #[test]
    fn no_truncate_short() {
        let s = "hello";
        assert_eq!(truncate_utf8(s, 2000), "hello");
    }
}
