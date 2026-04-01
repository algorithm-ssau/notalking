use std::{
    collections::{HashMap, VecDeque},
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant},
};

use axum::{
    Json,
    extract::{Request, State, connect_info::ConnectInfo},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use tokio::sync::Mutex;

use super::state::AppState;

#[derive(Clone)]
pub struct InMemoryRateLimiter {
    buckets: Arc<Mutex<HashMap<String, VecDeque<Instant>>>>,
    max_requests: usize,
    window: Duration,
}

impl InMemoryRateLimiter {
    pub fn new(max_requests: usize, window: Duration) -> Self {
        Self {
            buckets: Arc::new(Mutex::new(HashMap::new())),
            max_requests,
            window,
        }
    }

    pub async fn allow(&self, key: &str) -> bool {
        let mut buckets = self.buckets.lock().await;
        let now = Instant::now();
        let bucket = buckets.entry(key.to_owned()).or_default();

        while let Some(oldest) = bucket.front() {
            if now.duration_since(*oldest) > self.window {
                bucket.pop_front();
            } else {
                break;
            }
        }

        if bucket.len() >= self.max_requests {
            return false;
        }

        bucket.push_back(now);
        true
    }
}

#[derive(Serialize)]
struct RateLimitErrorResponse {
    error: String,
}

pub async fn auth_rate_limit(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let key = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|connect| connect.0.ip().to_string())
        .unwrap_or_else(|| "unknown".to_owned());

    if !state.rate_limiter.allow(&key).await {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(RateLimitErrorResponse {
                error: "too many requests, try again later".to_owned(),
            }),
        )
            .into_response();
    }

    next.run(request).await
}
