use std::{
    collections::{HashMap, VecDeque},
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant},
};

use axum::{
    Json,
    extract::{Request, State, connect_info::ConnectInfo},
    http::{HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use tokio::sync::Mutex;

use crate::config::CoreConfig;

use super::state::AppState;

#[derive(Clone)]
pub enum RateLimiterHandle {
    Memory(InMemoryRateLimiter),
    Redis(RedisRateLimiter),
}

impl RateLimiterHandle {
    pub async fn new_async(config: &CoreConfig) -> Self {
        if let Some(url) = config.redis_url.as_ref() {
            match redis::Client::open(url.as_str()) {
                Ok(client) => match redis::aio::ConnectionManager::new(client).await {
                    Ok(manager) => {
                        return Self::Redis(RedisRateLimiter {
                            client: manager,
                            max_requests: 60,
                            window_secs: 60,
                        });
                    }
                    Err(e) => tracing::warn!(error = %e, "redis connection failed for rate limiting"),
                },
                Err(e) => tracing::warn!(error = %e, "invalid redis URL for rate limiting"),
            }
            tracing::warn!("falling back to in-memory rate limiting");
        }
        Self::Memory(InMemoryRateLimiter::new(60, Duration::from_secs(60)))
    }

    pub async fn allow(&self, key: &str) -> bool {
        match self {
            RateLimiterHandle::Memory(m) => m.allow(key).await,
            RateLimiterHandle::Redis(r) => r.allow(key).await,
        }
    }
}

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

#[derive(Clone)]
pub struct RedisRateLimiter {
    client: redis::aio::ConnectionManager,
    max_requests: u32,
    window_secs: u64,
}

impl RedisRateLimiter {
    async fn allow(&self, key: &str) -> bool {
        let mut conn = self.client.clone();
        let k = format!("rl:core:{key}");
        let n: i64 = match redis::cmd("INCR")
            .arg(&k)
            .query_async(&mut conn)
            .await
        {
            Ok(n) => n,
            Err(_) => return true,
        };
        if n == 1 {
            let _: Result<(), _> = redis::cmd("EXPIRE")
                .arg(&k)
                .arg(self.window_secs as i64)
                .query_async(&mut conn)
                .await;
        }
        n <= self.max_requests as i64
    }
}

#[derive(Serialize)]
struct RateLimitErrorResponse {
    code: &'static str,
    message: String,
    details: Option<serde_json::Value>,
}

pub async fn global_rate_limit(
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
        let retry = HeaderValue::from_static("60");
        return (
            StatusCode::TOO_MANY_REQUESTS,
            [(
                axum::http::header::RETRY_AFTER,
                retry,
            )],
            Json(RateLimitErrorResponse {
                code: "rate_limited",
                message: "too many requests, try again later".to_owned(),
                details: None,
            }),
        )
            .into_response();
    }

    next.run(request).await
}

pub async fn auth_rate_limit(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let key = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|connect| format!("auth:{}", connect.0.ip()))
        .unwrap_or_else(|| "auth:unknown".to_owned());

    if !state.rate_limiter.allow(&key).await {
        let retry = HeaderValue::from_static("60");
        return (
            StatusCode::TOO_MANY_REQUESTS,
            [(
                axum::http::header::RETRY_AFTER,
                retry,
            )],
            Json(RateLimitErrorResponse {
                code: "rate_limited",
                message: "too many requests, try again later".to_owned(),
                details: None,
            }),
        )
            .into_response();
    }

    next.run(request).await
}
