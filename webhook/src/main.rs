mod events;
mod signature;
mod trailers;
mod trigger;

use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::Router;
use tokio::sync::Semaphore;

use crate::trigger::TriggerClient;

const MAX_BODY_SIZE: usize = 256 * 1024; // 256 KB
const MAX_CONCURRENT_DISPATCHES: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Forge {
    GitHub,
    Gitea,
}

impl Forge {
    fn signature_header(self) -> &'static str {
        match self {
            Forge::GitHub => "x-hub-signature-256",
            Forge::Gitea => "x-gitea-signature",
        }
    }

    fn event_header(self) -> &'static str {
        match self {
            Forge::GitHub => "x-github-event",
            Forge::Gitea => "x-gitea-event",
        }
    }
}

struct AppState {
    webhook_secret: String,
    forge: Forge,
    trigger: TriggerClient,
    dispatch_semaphore: Semaphore,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "bitswell_webhook=info".parse().unwrap()),
        )
        .init();

    let webhook_secret =
        std::env::var("WEBHOOK_SECRET").expect("WEBHOOK_SECRET env var required");
    let repo_path = std::env::var("REPO_PATH").unwrap_or_else(|_| "/repo".into());
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".into());

    let forge = match std::env::var("FORGE").unwrap_or_else(|_| "github".into()).as_str() {
        "gitea" => Forge::Gitea,
        _ => Forge::GitHub,
    };

    tracing::info!(forge = ?forge, "configured forge");

    let state = Arc::new(AppState {
        webhook_secret,
        forge,
        trigger: TriggerClient::new(repo_path.into()),
        dispatch_semaphore: Semaphore::new(MAX_CONCURRENT_DISPATCHES),
    });

    let app = Router::new()
        .route("/health", get(health))
        .route("/webhook", post(webhook))
        .layer(DefaultBodyLimit::max(MAX_BODY_SIZE))
        .with_state(state);

    let addr = format!("0.0.0.0:{port}");
    tracing::info!("listening on {addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

    // Graceful shutdown on SIGTERM/SIGINT
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();

    tracing::info!("server shut down");
}

async fn shutdown_signal() {
    let ctrl_c = tokio::signal::ctrl_c();
    #[cfg(unix)]
    {
        let mut sigterm =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).unwrap();
        tokio::select! {
            _ = ctrl_c => tracing::info!("received SIGINT, shutting down"),
            _ = sigterm.recv() => tracing::info!("received SIGTERM, shutting down"),
        }
    }
    #[cfg(not(unix))]
    {
        ctrl_c.await.ok();
        tracing::info!("received SIGINT, shutting down");
    }
}

async fn health() -> &'static str {
    "ok"
}

/// Acquire a dispatch permit, log if at capacity.
async fn acquire_permit(state: &AppState) -> Option<tokio::sync::SemaphorePermit<'_>> {
    match state.dispatch_semaphore.acquire().await {
        Ok(p) => Some(p),
        Err(_) => {
            tracing::error!("dispatch semaphore closed");
            None
        }
    }
}

async fn webhook(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let forge = state.forge;

    // Extract signature header (forge-specific)
    let signature = match headers
        .get(forge.signature_header())
        .and_then(|v| v.to_str().ok())
    {
        Some(s) => s,
        None => {
            tracing::warn!(header = forge.signature_header(), "missing signature header");
            return StatusCode::UNAUTHORIZED;
        }
    };

    // Verify HMAC
    if !signature::verify(signature, &state.webhook_secret, &body, forge) {
        tracing::warn!("invalid webhook signature");
        return StatusCode::UNAUTHORIZED;
    }

    // After signature verification, always return 200.
    // Parse errors are logged internally — don't trigger retries.
    let event_type = match headers
        .get(forge.event_header())
        .and_then(|v| v.to_str().ok())
    {
        Some(e) => e.to_string(),
        None => {
            tracing::warn!(header = forge.event_header(), "missing event header");
            return StatusCode::OK;
        }
    };

    tracing::info!(event = %event_type, ?forge, "received webhook event");

    // Dispatch to handler based on event type.
    // Handlers run in a spawned task with bounded concurrency.
    match event_type.as_str() {
        "push" => match serde_json::from_slice::<events::PushEvent>(&body) {
            Ok(event) => {
                let state = state.clone();
                tokio::spawn(async move {
                    let _permit = acquire_permit(&state).await;
                    events::handle_push(event, &state.trigger).await;
                });
            }
            Err(e) => tracing::error!(error = %e, "failed to parse push event"),
        },
        "issues" => match serde_json::from_slice::<events::IssuesEvent>(&body) {
            Ok(event) => {
                let state = state.clone();
                tokio::spawn(async move {
                    let _permit = acquire_permit(&state).await;
                    events::handle_issue_opened(event, &state.trigger).await;
                });
            }
            Err(e) => tracing::error!(error = %e, "failed to parse issues event"),
        },
        "issue_comment" => match serde_json::from_slice::<events::IssueCommentEvent>(&body) {
            Ok(event) => {
                let state = state.clone();
                tokio::spawn(async move {
                    let _permit = acquire_permit(&state).await;
                    events::handle_issue_comment(event, &state.trigger).await;
                });
            }
            Err(e) => tracing::error!(error = %e, "failed to parse issue_comment event"),
        },
        _ => {
            tracing::debug!(event = %event_type, "ignoring unhandled event type");
        }
    }

    StatusCode::OK
}
