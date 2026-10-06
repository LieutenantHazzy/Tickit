mod auth;
mod config;
mod db;
mod error;
mod handlers;
mod mailer;
mod models;
mod scheduler;
mod static_files;

use crate::{
    auth::session::RateLimitStore,
    config::Config,
    db::init_db,
    handlers::auth::AuthState,
    mailer::Mailer,
    static_files::{spa_fallback, static_router},
};
use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::{self, Next},
    routing::get,
    Router,
};
use std::{collections::HashMap, net::SocketAddr, sync::Arc, sync::Mutex};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Clone)]
pub struct AppState {
    pub mailer: Mailer,
}

async fn error_middleware(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> axum::response::Response {
    let path = req.uri().path().to_string();
    let method = req.method().to_string();
    let headers: HeaderMap = req.headers().clone();
    let resp = next.run(req).await;
    if resp.status().is_server_error() {
        let ua = headers
            .get("user-agent")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown");
        let ip = headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(',').next())
            .unwrap_or("unknown");
        let body = format!(
            "<h2>Error occurred</h2><p>Method: {}</p><p>Path: {}</p><p>IP: {}</p><p>User-Agent: {}</p>",
            method, path, ip, ua
        );
        state.mailer.send_error("Tickit - Error", &body).await;
    }
    resp
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::load()?;
    let pool = init_db(&config).await?;
    let mailer = Mailer::new(&config);

    let _scheduler =
        scheduler::start_scheduler(pool.clone(), config.clone(), mailer.clone()).await?;

    let rate_limit: RateLimitStore = Arc::new(Mutex::new(HashMap::new()));
    let auth_state = AuthState {
        pool: pool.clone(),
        config: config.clone(),
        rate_limit: rate_limit.clone(),
    };
    let app_state = AppState {
        mailer: mailer.clone(),
    };

    let api = Router::new()
        .nest("/auth", handlers::auth::router().with_state(auth_state))
        .nest(
            "/lists",
            handlers::lists::router().with_state(AuthState {
                pool: pool.clone(),
                config: config.clone(),
                rate_limit: rate_limit.clone(),
            }),
        )
        .nest(
            "/todos",
            handlers::todos::router().with_state(AuthState {
                pool: pool.clone(),
                config: config.clone(),
                rate_limit: rate_limit.clone(),
            }),
        )
        .route("/healthz", get(|| async { StatusCode::OK }));

    let app = Router::new()
        .nest("/api", api)
        .merge(static_router())
        .layer(middleware::from_fn_with_state(app_state, error_middleware))
        .layer(TraceLayer::new_for_http())
        .fallback(spa_fallback);

    let addr = SocketAddr::new(config.bind_addr.parse()?, config.bind_port);
    tracing::info!("listening on {}", addr);
    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;
    Ok(())
}
