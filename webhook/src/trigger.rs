use std::time::Duration;

use reqwest::Client;
use serde::Serialize;

use crate::trailers::AssignedCommit;

const TRIGGER_API_BASE: &str = "https://api.claude.ai/v1/code/triggers";
const MAX_RETRIES: u32 = 1;
const RETRY_DELAY: Duration = Duration::from_secs(2);
const MAX_COMMENT_CHARS: usize = 2000;

#[derive(Debug, Serialize)]
struct TriggerRunBody {
    input: String,
}

/// Structured dispatch payload sent as JSON inside the trigger input.
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
    http: Client,
    trigger_id: String,
    api_key: String,
}

impl TriggerClient {
    pub fn new(trigger_id: String, api_key: String) -> Self {
        Self {
            http: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("failed to build HTTP client"),
            trigger_id,
            api_key,
        }
    }

    /// Dispatch a LOOM assignment via RemoteTrigger with full trailer data.
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
        let input = serde_json::to_string_pretty(&payload)
            .unwrap_or_else(|_| format!("{{\"event\":\"push.assigned\",\"commit_sha\":\"{commit_sha}\"}}"));
        self.run_with_retry(&input).await
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
        let input = serde_json::to_string_pretty(&payload)
            .unwrap_or_else(|_| format!("{{\"event\":\"issues.opened\",\"number\":{number}}}"));
        self.run_with_retry(&input).await
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
        let input = serde_json::to_string_pretty(&payload)
            .unwrap_or_else(|_| format!("{{\"event\":\"issue_comment.created\",\"issue_number\":{issue_number}}}"));
        self.run_with_retry(&input).await
    }

    async fn run_with_retry(&self, input: &str) -> Result<(), TriggerError> {
        let mut last_err = None;
        for attempt in 0..=MAX_RETRIES {
            if attempt > 0 {
                tracing::info!(attempt, "retrying trigger dispatch");
                tokio::time::sleep(RETRY_DELAY * attempt).await;
            }
            match self.run(input).await {
                Ok(()) => return Ok(()),
                Err(e) => {
                    if e.is_retryable() && attempt < MAX_RETRIES {
                        tracing::warn!(attempt, error = %e, "retryable dispatch error");
                        last_err = Some(e);
                        continue;
                    }
                    return Err(e);
                }
            }
        }
        Err(last_err.expect("retry loop should have returned an error"))
    }

    async fn run(&self, input: &str) -> Result<(), TriggerError> {
        let url = format!("{}/{}/run", TRIGGER_API_BASE, self.trigger_id);

        let resp = self
            .http
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&TriggerRunBody {
                input: input.to_string(),
            })
            .send()
            .await
            .map_err(TriggerError::Http)?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(TriggerError::Api { status, body });
        }

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
    Http(reqwest::Error),
    Api {
        status: reqwest::StatusCode,
        body: String,
    },
}

impl TriggerError {
    fn is_retryable(&self) -> bool {
        match self {
            Self::Http(e) => e.is_timeout() || e.is_connect(),
            Self::Api { status, .. } => {
                *status == reqwest::StatusCode::TOO_MANY_REQUESTS
                    || status.is_server_error()
            }
        }
    }
}

impl std::fmt::Display for TriggerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Http(e) => write!(f, "HTTP error: {e}"),
            Self::Api { status, body } => write!(f, "API error {status}: {body}"),
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
        // Each crab emoji is 4 bytes. 501 crabs, truncate at 500 chars.
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

    #[test]
    fn retryable_errors() {
        let api_429 = TriggerError::Api {
            status: reqwest::StatusCode::TOO_MANY_REQUESTS,
            body: String::new(),
        };
        assert!(api_429.is_retryable());

        let api_503 = TriggerError::Api {
            status: reqwest::StatusCode::SERVICE_UNAVAILABLE,
            body: String::new(),
        };
        assert!(api_503.is_retryable());

        let api_400 = TriggerError::Api {
            status: reqwest::StatusCode::BAD_REQUEST,
            body: String::new(),
        };
        assert!(!api_400.is_retryable());
    }
}
