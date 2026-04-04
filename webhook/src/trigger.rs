use reqwest::Client;
use serde::Serialize;

const TRIGGER_API_BASE: &str = "https://api.claude.ai/v1/code/triggers";

#[derive(Debug, Serialize)]
struct TriggerRunBody {
    input: String,
}

pub struct TriggerClient {
    http: Client,
    trigger_id: String,
    api_key: String,
}

impl TriggerClient {
    pub fn new(trigger_id: String, api_key: String) -> Self {
        Self {
            http: Client::new(),
            trigger_id,
            api_key,
        }
    }

    /// Dispatch a LOOM assignment via RemoteTrigger.
    pub async fn dispatch_push(
        &self,
        commit_sha: &str,
        branch: &str,
    ) -> Result<(), TriggerError> {
        let input = format!(
            "A push event was received. An ASSIGNED commit was detected.\n\
             Run: loom-dispatch.sh --commit {commit_sha} --branch {branch}"
        );
        self.run(&input).await
    }

    /// Notify about a new issue.
    pub async fn dispatch_issue(
        &self,
        number: u64,
        title: &str,
        author: &str,
    ) -> Result<(), TriggerError> {
        let input = format!(
            "A new GitHub issue was opened.\n\
             Issue #{number}: {title}\n\
             Author: {author}\n\
             Read the issue, triage it, and create tasks if appropriate."
        );
        self.run(&input).await
    }

    /// Notify about a new issue comment.
    pub async fn dispatch_comment(
        &self,
        issue_number: u64,
        author: &str,
        body: &str,
    ) -> Result<(), TriggerError> {
        // Truncate comment body to avoid oversized payloads
        let truncated = if body.len() > 2000 {
            format!("{}... (truncated)", &body[..2000])
        } else {
            body.to_string()
        };
        let input = format!(
            "A new comment was posted on issue #{issue_number}.\n\
             Author: {author}\n\
             Comment: {truncated}\n\
             Read the full context and decide if action is needed."
        );
        self.run(&input).await
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

#[derive(Debug)]
pub enum TriggerError {
    Http(reqwest::Error),
    Api {
        status: reqwest::StatusCode,
        body: String,
    },
}

impl std::fmt::Display for TriggerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Http(e) => write!(f, "HTTP error: {e}"),
            Self::Api { status, body } => write!(f, "API error {status}: {body}"),
        }
    }
}
