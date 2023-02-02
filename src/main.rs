use std::{collections::HashMap, future::IntoFuture, net::SocketAddr, sync::Arc};

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use nomad_webhook::{webhook, NomadConfig, Task};
use tracing::instrument;
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt};

#[derive(Debug)]
struct AppState {
    config: NomadConfig,
    client: reqwest::Client,
    task_map: HashMap<String, HashMap<String, Task>>,
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

    let map = {
        let mut tmp = HashMap::new();
        tmp.insert("infra".to_string(), {
            let mut infra = HashMap::new();
            infra.insert(
                "docs-latest".to_string(),
                Task::RestartJob {
                    id: "docs".to_string(),
                },
            );
            infra
        });
        tmp
    };

    let app = Router::new()
        .route("/:name", post(webhook))
        .with_state(Arc::new(AppState {
            config,
            client,
            task_map: map,
        }));

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[instrument(skip(state, content))]
async fn webhook(
    Path(name): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(content): Json<serde_json::Value>,
) -> impl IntoResponse {
    let req_endpoint = match state.task_map.get(&name) {
        Some(endp) => endp,
        None => {
            tracing::error!("Unexpected Webhook endpoint '{name}'");

            return axum::response::Response::builder()
                .status(axum::http::StatusCode::NOT_FOUND)
                .body(String::new())
                .unwrap();
        }
    };

    tracing::info!("Webhook '{name}'");

    let payload = match webhook::GithubPackagePayload::into_package(content) {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("Parsing Payload {:?}", e);

            return axum::response::Response::builder()
                .status(axum::http::StatusCode::BAD_REQUEST)
                .body(String::new())
                .unwrap();
        }
    };

    let tag = &payload.package.package_version.container_metadata.tag;

    let task = match req_endpoint.get(&tag.name) {
        Some(t) => t,
        None => {
            tracing::error!("Payload: {:#?}", payload);

            return axum::response::Response::builder()
                .status(axum::http::StatusCode::BAD_REQUEST)
                .body(String::new())
                .unwrap();
        }
    };

    tracing::info!("Performing Task: {:?}", task);

    task.perform(&state.config, &state.client).await;

    axum::response::Response::builder()
        .status(200)
        .body(String::new())
        .unwrap()
}
