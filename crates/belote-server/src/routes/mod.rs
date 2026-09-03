//! Routage HTTP.

pub mod auth_routes;
pub mod games;
pub mod stats;
pub mod tables;
pub mod ws;

use axum::http::{header, HeaderValue, Method};
use axum::routing::{get, post};
use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::AppState;

pub fn router(state: AppState) -> Router {
    // Origine explicite : `*` est interdit des lors qu'on envoie des cookies.
    let cors = CorsLayer::new()
        .allow_origin(
            state
                .config
                .allowed_origin
                .parse::<HeaderValue>()
                .expect("ALLOWED_ORIGIN doit etre une origine valide"),
        )
        .allow_credentials(true)
        .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::OPTIONS])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE]);

    Router::new()
        .route("/health", get(health))
        .route("/api/auth/register", post(auth_routes::register))
        .route("/api/auth/login", post(auth_routes::login))
        .route("/api/auth/refresh", post(auth_routes::refresh))
        .route("/api/auth/logout", post(auth_routes::logout))
        .route("/api/me", get(auth_routes::me))
        .route("/api/tables", post(tables::create))
        .route("/api/tables/{code}", get(tables::show))
        .route("/api/tables/{code}/join", post(tables::join))
        .route("/api/games", get(games::list))
        .route("/api/games/{id}", get(games::show))
        .route("/api/stats", get(stats::show))
        .route("/api/ws-ticket", post(ws::issue_ticket))
        .route("/ws", get(ws::connect))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Sonde de vie : Render l'interroge pour savoir si le service est pret.
async fn health() -> &'static str {
    "ok"
}
