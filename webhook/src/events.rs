use serde::Deserialize;

use crate::trailers::is_assigned_commit;
use crate::trigger::TriggerClient;

// --- Push event payload (subset) ---

#[derive(Debug, Deserialize)]
pub struct PushEvent {
    #[serde(rename = "ref")]
    pub git_ref: String,
    pub commits: Vec<PushCommit>,
}

#[derive(Debug, Deserialize)]
pub struct PushCommit {
    pub id: String,
    pub message: String,
}

// --- Issues event payload (subset) ---

#[derive(Debug, Deserialize)]
pub struct IssuesEvent {
    pub action: String,
    pub issue: Issue,
}

#[derive(Debug, Deserialize)]
pub struct Issue {
    pub number: u64,
    pub title: String,
    pub user: User,
}

// --- Issue comment event payload (subset) ---

#[derive(Debug, Deserialize)]
pub struct IssueCommentEvent {
    pub action: String,
    pub issue: IssueRef,
    pub comment: Comment,
}

#[derive(Debug, Deserialize)]
pub struct IssueRef {
    pub number: u64,
}

#[derive(Debug, Deserialize)]
pub struct Comment {
    pub user: User,
    pub body: String,
}

#[derive(Debug, Deserialize)]
pub struct User {
    pub login: String,
}

// --- Handlers ---

/// Handle a push event. Returns the number of dispatches triggered.
pub async fn handle_push(event: PushEvent, trigger: &TriggerClient) -> usize {
    let branch = event.git_ref.strip_prefix("refs/heads/").unwrap_or(&event.git_ref);

    if !branch.starts_with("loom/") {
        tracing::debug!(branch, "ignoring push to non-loom branch");
        return 0;
    }

    let mut dispatched = 0;
    for commit in &event.commits {
        // Validate commit SHA is hex
        if !commit.id.chars().all(|c| c.is_ascii_hexdigit()) {
            tracing::warn!(sha = %commit.id, "invalid commit SHA format, skipping");
            continue;
        }

        if is_assigned_commit(&commit.message) {
            tracing::info!(sha = %commit.id, branch, "ASSIGNED commit detected, dispatching");
            match trigger.dispatch_push(&commit.id, branch).await {
                Ok(()) => dispatched += 1,
                Err(e) => tracing::error!(sha = %commit.id, error = %e, "dispatch failed"),
            }
        }
    }

    dispatched
}

/// Handle an issues.opened event.
pub async fn handle_issue_opened(event: IssuesEvent, trigger: &TriggerClient) {
    if event.action != "opened" {
        return;
    }
    tracing::info!(
        number = event.issue.number,
        title = %event.issue.title,
        "new issue opened, dispatching"
    );
    if let Err(e) = trigger
        .dispatch_issue(event.issue.number, &event.issue.title, &event.issue.user.login)
        .await
    {
        tracing::error!(number = event.issue.number, error = %e, "issue dispatch failed");
    }
}

/// Handle an issue_comment.created event.
pub async fn handle_issue_comment(event: IssueCommentEvent, trigger: &TriggerClient) {
    if event.action != "created" {
        return;
    }
    tracing::info!(
        issue = event.issue.number,
        author = %event.comment.user.login,
        "new issue comment, dispatching"
    );
    if let Err(e) = trigger
        .dispatch_comment(
            event.issue.number,
            &event.comment.user.login,
            &event.comment.body,
        )
        .await
    {
        tracing::error!(issue = event.issue.number, error = %e, "comment dispatch failed");
    }
}
