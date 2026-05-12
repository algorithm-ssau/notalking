use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;

mod auth;
mod config;
mod db;
mod embedding;
mod grpc_server;
mod http;
mod mcp;
mod nats_bridge;
mod note;
mod persist;
mod user;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Arc::new(config::CoreConfig::load());

    let env_filter = tracing_subscriber::EnvFilter::new(&config.log_filter);
    let subscriber = tracing_subscriber::fmt().with_env_filter(env_filter);
    if matches!(config.environment, config::Environment::Prod) {
        subscriber.json().init();
    } else {
        subscriber.init();
    }

    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    std::fs::create_dir_all(manifest.join("data")).ok();

    let db = Arc::new(
        db::Db::connect(&config.database_url, manifest)
            .await
            .map_err(|e| anyhow::anyhow!("database connect: {e}"))?,
    );
    db.run_migrations_from_crate_root(manifest)
        .await
        .map_err(|e| anyhow::anyhow!("migrations: {e}"))?;

    let state = http::state::AppState::build(db, config.clone()).await?;

    if let Some(addr) = config.metrics_bind {
        metrics_exporter_prometheus::PrometheusBuilder::new()
            .with_http_listener(addr)
            .install()
            .map_err(|e| anyhow::anyhow!("metrics: {e}"))?;
        tracing::info!(%addr, "prometheus metrics");
    }

    nats_bridge::connect_optional(config.nats_url.as_deref()).await;

    if let Some(grpc_addr) = config.grpc_bind {
        let notes = state.note_store.clone();
        let note_service = state.note.clone();
        let embedding = state.embedding.clone();
        tokio::spawn(async move {
            if let Err(e) = grpc_server::serve_grpc(grpc_addr, notes, note_service, embedding).await
            {
                tracing::error!(error = %e, "gRPC server exited");
            }
        });
        tracing::info!(%grpc_addr, "gRPC CoreBridge");
    }

    let app = http::router::create_http_router(state);

    tracing::info!(addr = %config.http_bind, "HTTP listening");
    let listener = tokio::net::TcpListener::bind(config.http_bind).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}
