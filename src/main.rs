use std::{net::SocketAddr, path::PathBuf, sync::Arc};

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use nomad_webhook::{webhook, Config, NomadConfig};
use tracing::instrument;
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt};

#[derive(Debug)]
struct AppState {
    config: NomadConfig,
    client: reqwest::Client,
    task_map: Config,
}

#[tokio::main]
async fn main() {
    let machine_log = std::env::var("LOG_MACHINE").is_ok();

    let current_path = std::env::current_dir().unwrap();
    let config_path = match std::env::var("CONF_FILE") {
        Ok(f) => PathBuf::from(f),
        Err(_) => current_path.join("config.json"),
    };

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

    let nomad_config = NomadConfig::new(format!("http://{address}:{port}"));

    let client = reqwest::Client::builder().build().unwrap();

    let config = Config::load(&config_path).await.unwrap();

    let app = Router::new()
        .route("/:name", post(webhook))
        .with_state(Arc::new(AppState {
            config: nomad_config,
            client,
            task_map: config,
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
    let req_endpoint = match state.task_map.endpoints.get(&name) {
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
