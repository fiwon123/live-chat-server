//! live-chat-server API server entry point.

use live_chat_server::{AppState, config::Config};
use sqlx::PgPool;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                tracing_subscriber::EnvFilter::new("live_chat_server=info,tower_http=info")
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::from_env();
    if config.uses_default_jwt_secret() {
        tracing::warn!(
            "JWT_SECRET is unset; using the committed insecure default. \
             Set JWT_SECRET to a strong random value before deploying."
        );
    }
    // The full DATABASE_URL may contain credentials; log only the port.
    tracing::info!(port = config.port, "starting live-chat-server");

    let pool = PgPool::connect(&config.database_url).await?;
    // Apply pending migrations on startup so the dev/compose flow never needs a
    // manual `sqlx migrate run`.
    sqlx::migrate!("./migrations").run(&pool).await?;
    tracing::info!("database migrations applied");

    let port = config.port;
    let app = live_chat_server::build_router(AppState::new(pool, config)).layer(
        tower_http::trace::TraceLayer::new_for_http().make_span_with(
            |request: &axum::extract::Request| {
                // The WebSocket handshake carries the JWT as a query parameter;
                // log only the path, never the query string.
                tracing::debug_span!(
                    "http_request",
                    method = %request.method(),
                    path = %request.uri().path(),
                    status = tracing::field::Empty,
                )
            },
        ),
    );

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await?;
    tracing::info!("listening on http://0.0.0.0:{}", port);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

/// Wait for SIGINT/SIGTERM so `axum::serve` can shut down gracefully.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!("shutdown signal received");
}
