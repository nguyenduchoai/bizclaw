//! A2A Protocol Routes - Google Agent-to-Agent Communication

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use bizclaw_agent::a2a::{A2AServer, GetTaskResponse, SendTaskRequest, SendTaskResponse, TaskEvent};
use std::sync::Arc;
use tokio_stream::StreamExt;
use tower_http::cors::{Any, CorsLayer};

#[derive(Clone)]
pub struct A2AState {
    pub server: Arc<A2AServer>,
}

pub async fn create_a2a_routes(server: A2AServer) -> Router {
    let state = A2AState {
        server: Arc::new(server),
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/.well-known/agent.json", get(agent_card))
        .route("/a2a/v1/tasks/send", post(send_task))
        .route("/a2a/v1/tasks/{id}", get(get_task))
        .route("/a2a/v1/tasks/{id}/stream", get(stream_task))
        .with_state(state)
        .layer(cors)
}

async fn agent_card(State(state): State<A2AState>) -> Json<serde_json::Value> {
    let card = state.server.agent_card();
    Json(serde_json::to_value(card).unwrap())
}

async fn send_task(
    State(state): State<A2AState>,
    Json(request): Json<SendTaskRequest>,
) -> Result<Json<SendTaskResponse>, StatusCode> {
    let response = state.server.send_task(request).await;
    Ok(Json(response))
}

async fn get_task(
    State(state): State<A2AState>,
    Path(task_id): Path<String>,
) -> Result<Json<GetTaskResponse>, StatusCode> {
    state
        .server
        .get_task(&task_id)
        .await
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

async fn stream_task(
    State(state): State<A2AState>,
    Path(task_id): Path<String>,
) -> impl axum::response::IntoResponse {
    let mut rx = state.server.stream_events(&task_id).await;
    
    axum::response::Sse::new(async_stream::stream! {
        while let Some(event) = rx.recv().await {
            yield Ok::<_, std::convert::Infallible>(axum::response::Event::default()
                .json_data(event)
                .unwrap());
        }
    })
}
