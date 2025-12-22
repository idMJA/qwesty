use crate::models::{Quest, StoredQuest};
use crate::services::{storage, webhook::WebhookNotifier};
use axum::http::StatusCode;
use axum::{extract::State, routing::post, Json, Router};
use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::Arc;

#[derive(Clone)]
pub struct IngestState {
    pub accept_token: Option<String>,
    pub notifiers: Arc<Vec<WebhookNotifier>>,
}

#[derive(Debug, Deserialize)]
pub struct IngestPayload {
    pub region: String,
    pub quests: Vec<Quest>,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct IngestResponse {
    pub accepted: usize,
    pub deduped: usize,
}

/// Start the ingest server for receiving quests from agents.
///
/// # Panics
/// Panics if TCP listener binding fails.
pub async fn start_server(
    accept_token: Option<String>,
    port: u16,
    notifiers: Vec<WebhookNotifier>,
) {
    let state = IngestState {
        accept_token,
        notifiers: Arc::new(notifiers),
    };
    let app = Router::new()
        .route("/ingest", post(ingest_handler))
        .route("/health", axum::routing::get(health_handler))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("collector ingest server listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind ingest listener");
    axum::serve(listener, app)
        .await
        .expect("ingest server failed");
}

async fn health_handler() -> &'static str {
    "OK"
}

async fn ingest_handler(
    State(state): State<IngestState>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<IngestPayload>,
) -> Result<Json<IngestResponse>, (StatusCode, String)> {
    // auth
    if let Some(expected) = &state.accept_token {
        let Some(auth) = headers.get(axum::http::header::AUTHORIZATION) else {
            return Err((
                StatusCode::UNAUTHORIZED,
                "missing Authorization header".to_string(),
            ));
        };
        let Ok(auth_str) = auth.to_str() else {
            return Err((
                StatusCode::UNAUTHORIZED,
                "invalid Authorization header".to_string(),
            ));
        };
        if !auth_str.starts_with("Bearer ") || auth_str[7..] != *expected {
            return Err((StatusCode::UNAUTHORIZED, "invalid token".to_string()));
        }
    }

    info!(
        "received ingest from source: {:?} for region: {}",
        payload.source, payload.region
    );

    // convert quests to StoredQuest and prefix id with region for regional dedupe
    let mut stored = storage::load_stored_quests();
    let before = stored.len();

    let mut new_entries: Vec<StoredQuest> = Vec::new();
    for q in &payload.quests {
        let mut sq = StoredQuest::from(q);
        sq.id = format!("{}:{}", payload.region, sq.id);
        new_entries.push(sq);
    }

    let new_only = storage::find_new_quests(&new_entries, &stored);
    if new_only.is_empty() {
        return Ok(Json(IngestResponse {
            accepted: 0,
            deduped: 0,
        }));
    }

    if !state.notifiers.is_empty() {
        let mut new_quest_ids: Vec<String> = new_only
            .iter()
            .map(|q| {
                q.id.split(':')
                    .next_back()
                    .unwrap_or(q.id.as_str())
                    .to_string()
            })
            .collect();

        // build base-id set from already stored to suppress cross-region duplicates
        let seen_base: HashSet<String> = stored
            .iter()
            .map(|q| {
                q.id.split(':')
                    .next_back()
                    .unwrap_or(q.id.as_str())
                    .to_string()
            })
            .collect();
        new_quest_ids.retain(|id| !seen_base.contains(id));

        let full_new_quests: Vec<_> = payload
            .quests
            .iter()
            .filter(|q| new_quest_ids.contains(&q.config.id))
            .cloned()
            .collect();

        if !full_new_quests.is_empty() {
            for notifier in state.notifiers.iter() {
                if let Err(e) = notifier.notify_full(&full_new_quests).await {
                    warn!("failed to send notification for ingested quests: {e}");
                }
            }
            info!(
                "sent notifications for {} new ingested quests from region {}",
                full_new_quests.len(),
                payload.region
            );
        }
    }

    stored.extend(new_only.iter().cloned());
    let merged = crate::utils::dedupe_by_key(&stored, |q| q.id.clone());
    storage::save_quests(&merged)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(IngestResponse {
        accepted: new_only.len(),
        deduped: merged.len().saturating_sub(before),
    }))
}
