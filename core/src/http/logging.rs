use std::{
    net::SocketAddr,
    sync::atomic::{AtomicU64, Ordering},
    time::Instant,
};

use axum::{
    extract::{Request, connect_info::ConnectInfo},
    http::{HeaderValue, header::USER_AGENT},
    middleware::Next,
    response::Response,
};

static REQUEST_SEQ: AtomicU64 = AtomicU64::new(1);

fn user_agent_from_header(value: Option<&HeaderValue>) -> &str {
    match value.and_then(|header| header.to_str().ok()) {
        Some(ua) if !ua.is_empty() => ua,
        _ => "-",
    }
}

pub async fn request_logging_middleware(request: Request, next: Next) -> Response {
    let request_id = REQUEST_SEQ.fetch_add(1, Ordering::Relaxed);
    let method = request.method().to_string();
    let uri = request.uri().to_string();
    let ip = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|connect| connect.0.ip().to_string())
        .unwrap_or_else(|| "unknown".to_owned());
    let user_agent = user_agent_from_header(request.headers().get(USER_AGENT)).to_owned();

    let started_at = Instant::now();
    log::info!(
        "[req:{request_id}] started method={method} uri={uri} ip={ip} user_agent=\"{user_agent}\""
    );

    let response = next.run(request).await;
    let status = response.status();
    let elapsed_ms = started_at.elapsed().as_millis();

    if status.is_server_error() {
        log::error!(
            "[req:{request_id}] finished status={} elapsed_ms={} method={} uri={} ip={}",
            status.as_u16(),
            elapsed_ms,
            method,
            uri,
            ip
        );
    } else if status.is_client_error() {
        log::warn!(
            "[req:{request_id}] finished status={} elapsed_ms={} method={} uri={} ip={}",
            status.as_u16(),
            elapsed_ms,
            method,
            uri,
            ip
        );
    } else {
        log::info!(
            "[req:{request_id}] finished status={} elapsed_ms={} method={} uri={} ip={}",
            status.as_u16(),
            elapsed_ms,
            method,
            uri,
            ip
        );
    }

    response
}
