use std::{net::SocketAddr, sync::Arc};

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use nomad_webhook::{NomadConfig, Task};
use tracing::instrument;
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt};

#[derive(Debug)]
struct AppState {
    config: NomadConfig,
    client: reqwest::Client,
}

#[tokio::main]
async fn main() {
    let machine_log = std::env::var("LOG_MACHINE").is_ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "nomad_webhook=trace".into()),
        )
        .with((machine_log).then(|| tracing_subscriber::fmt::layer().json()))
        .with((!machine_log).then(|| tracing_subscriber::fmt::layer().pretty()))
        .init();

    let address = std::env::var("NOMAD_ADDR").unwrap_or_else(|_| "localhost".to_string());
    let port = std::env::var("NOMAD_PORT").unwrap_or_else(|_| "4646".to_string());

    let config = NomadConfig::new(format!("http://{address}:{port}"));

    let client = reqwest::Client::builder().build().unwrap();

    let app = Router::new()
        .route("/:name", post(webhook))
        .with_state(Arc::new(AppState { config, client }));

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[instrument(skip(name, state, content))]
async fn webhook(
    Path(name): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(content): Json<serde_json::Value>,
) -> impl IntoResponse {
    let task = Task::RestartJob {
        id: "docs".to_string(),
    };

    task.perform(&state.config, &state.client).await;

    tracing::info!("Webhook '{name}' with content: {:#?}", content);

    axum::response::Response::builder()
        .status(200)
        .body(String::new())
        .unwrap()
}
