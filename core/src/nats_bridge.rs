/// Optional NATS connection for async messaging (Intelligence <-> Core). Fails soft when URL missing.
pub async fn connect_optional(url: Option<&str>) {
    let Some(u) = url else {
        return;
    };
    if u.is_empty() {
        return;
    }
    match async_nats::connect(u).await {
        Ok(_client) => {
            tracing::info!(%u, "connected to NATS (subscriber not yet implemented)");
        }
        Err(e) => {
            tracing::warn!(error = %e, %u, "NATS unavailable; continuing without async bus");
        }
    }
}
