use axum::{
    extract::{State},
    routing::{get, post},
    Json, Router,
};
use beryl_common::{FirewallConfig, Stats};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::trace::TraceLayer;

// Re-export Router for use in main.rs
pub use crate::Router as AppRouter;

#[derive(Clone)]
pub struct AppState {
    pub router: Arc<RwLock<AppRouter>>,
}

#[derive(Serialize)]
pub struct StatusResponse {
    pub version: &'static str,
    pub mode: &'static str, // Placeholder for Phase 3
    pub services: ServicesStatus,
}

#[derive(Serialize)]
pub struct ServicesStatus {
    pub dhcp_server: &'static str,
    pub dns_server: &'static str,
    pub wifi: &'static str,
}

#[derive(Serialize)]
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
            dhcp_server: "stopped", // Phase 2
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

async fn get_config(State(state): State<AppState>) -> Json<FirewallConfig> {
    let router = state.router.read().await;
    // In a real app, we'd probably cache the config in AppRouter or read it from the file
    // For now, return default or empty since we don't store the config in memory in AppRouter yet
    // We should probably add config storage to AppRouter.
    // ...Refactor needed in Router struct...
    Json(FirewallConfig::default()) 
}

async fn put_config(
    State(state): State<AppState>,
    Json(config): Json<FirewallConfig>,
) -> Json<FirewallConfig> {
    let mut router = state.router.write().await;
    if let Err(e) = router.apply_config(&config) {
        tracing::error!("Failed to apply config via API: {}", e);
        // In production, return 500 or 400
    }
    // In a real app, we would write this back to the config file
    // For Phase 1, just applying it to eBPF maps is enough to demonstrate
    Json(config)
}
