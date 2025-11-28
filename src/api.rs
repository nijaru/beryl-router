use axum::{
    extract::{State},
    routing::{get},
    Json, Router,
};
use beryl_common::{Stats};
use beryl_config::Config;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::trace::TraceLayer;

// Re-export Router for use in main.rs
pub use crate::Router as AppRouter;

#[derive(Clone)]
pub struct AppState {
    pub router: Arc<RwLock<AppRouter>>,
}

#[derive(serde::Serialize)]
pub struct StatusResponse {
    pub version: &'static str,
    pub mode: &'static str, // Placeholder for Phase 3
    pub services: ServicesStatus,
}

#[derive(serde::Serialize)]
pub struct ServicesStatus {
    pub dhcp_server: &'static str,
    pub dns_server: &'static str,
    pub wifi: &'static str,
}

#[derive(serde::Serialize)]
pub struct StatsResponse {
    pub packets: Stats,
}

pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/status", get(status_handler))
        .route("/api/v1/stats", get(stats_handler))
        .route("/api/v1/config", get(get_config).put(put_config))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn status_handler() -> Json<StatusResponse> {
    Json(StatusResponse {
        version: env!("CARGO_PKG_VERSION"),
        mode: "router", // Hardcoded for Phase 1
        services: ServicesStatus {
            dhcp_server: "stopped", // Phase 2 (implemented but no status check yet)
            dns_server: "stopped",  // Phase 2
            wifi: "stopped",        // Phase 3
        },
    })
}

async fn stats_handler(State(state): State<AppState>) -> Json<StatsResponse> {
    let router = state.router.read().await;
    let stats = router.get_stats().unwrap_or_default();
    Json(StatsResponse { packets: stats })
}

async fn get_config(State(state): State<AppState>) -> Json<Option<Config>> {
    let router = state.router.read().await;
    Json(router.get_current_config())
}

async fn put_config(
    State(state): State<AppState>,
    Json(config): Json<Config>,
) -> Json<Config> {
    let mut router = state.router.write().await;
    
    if let Err(e) = router.apply_firewall_config(&config.firewall) {
        tracing::error!("Failed to apply firewall config: {}", e);
    }
    if let Err(e) = router.apply_dhcp_config(&config.dhcp).await {
        tracing::error!("Failed to apply DHCP config: {}", e);
    }
    
    // Note: We are not persisting the config to file here yet.
    // It will be lost on restart.
    
    Json(config)
}