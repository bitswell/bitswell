mod events;
mod signature;
mod trailers;
mod trigger;

use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::Router;

use crate::trigger::TriggerClient;

struct AppState {
    webhook_secret: String,
    trigger: TriggerClient,
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
    let trigger_id = std::env::var("TRIGGER_ID").expect("TRIGGER_ID env var required");
    let api_key = std::env::var("CLAUDE_API_KEY").expect("CLAUDE_API_KEY env var required");
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".into());

    let state = Arc::new(AppState {
        webhook_secret,
        trigger: TriggerClient::new(trigger_id, api_key),
    });

    let app = Router::new()
        .route("/health", get(health))
        .route("/webhook", post(webhook))
        .with_state(state);

    let addr = format!("0.0.0.0:{port}");
    tracing::info!("listening on {addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health() -> &'static str {
    "ok"
}

async fn webhook(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    // Extract signature header
    let signature = match headers
        .get("x-hub-signature-256")
        .and_then(|v| v.to_str().ok())
    {
        Some(s) => s,
        None => {
            tracing::warn!("missing x-hub-signature-256 header");
            return StatusCode::UNAUTHORIZED;
        }
    };

    // Verify HMAC
    if !signature::verify(signature, &state.webhook_secret, &body) {
        tracing::warn!("invalid webhook signature");
        return StatusCode::UNAUTHORIZED;
    }

    // Extract event type
    let event_type = match headers
        .get("x-github-event")
        .and_then(|v| v.to_str().ok())
    {
        Some(e) => e.to_string(),
        None => {
            tracing::warn!("missing x-github-event header");
            return StatusCode::BAD_REQUEST;
        }
    };

    tracing::info!(event = %event_type, "received webhook event");

    // Dispatch to handler based on event type.
    // Handlers run in a spawned task so we return 200 immediately.
    let trigger = &state.trigger;

    match event_type.as_str() {
        "push" => match serde_json::from_slice::<events::PushEvent>(&body) {
            Ok(event) => {
                let trigger = state.clone();
                tokio::spawn(async move {
                    events::handle_push(event, &trigger.trigger).await;
                });
            }
            Err(e) => {
                tracing::error!(error = %e, "failed to parse push event");
                return StatusCode::BAD_REQUEST;
            }
        },
        "issues" => match serde_json::from_slice::<events::IssuesEvent>(&body) {
            Ok(event) => {
                let trigger = state.clone();
                tokio::spawn(async move {
                    events::handle_issue_opened(event, &trigger.trigger).await;
                });
            }
            Err(e) => {
                tracing::error!(error = %e, "failed to parse issues event");
                return StatusCode::BAD_REQUEST;
            }
        },
        "issue_comment" => match serde_json::from_slice::<events::IssueCommentEvent>(&body) {
            Ok(event) => {
                let trigger = state.clone();
                tokio::spawn(async move {
                    events::handle_issue_comment(event, &trigger.trigger).await;
                });
            }
            Err(e) => {
                tracing::error!(error = %e, "failed to parse issue_comment event");
                return StatusCode::BAD_REQUEST;
            }
        },
        _ => {
            tracing::debug!(event = %event_type, "ignoring unhandled event type");
        }
    }

    // Suppress unused variable warning — trigger is borrowed via state.clone() above
    let _ = trigger;

    StatusCode::OK
}
