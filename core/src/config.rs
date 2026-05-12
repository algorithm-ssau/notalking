use std::net::SocketAddr;
use std::str::FromStr;

use clap::Parser;

/// Embedding regeneration strategy -- locked at process start (SPEC 4.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddingRegenerationMode {
    OnEachBlockPatch,
    AfterQuietPeriodSinceLastPatch { idle_ms: u64 },
}

#[derive(Debug, Clone)]
pub struct CoreConfig {
    pub environment: Environment,
    pub http_bind: SocketAddr,
    pub metrics_bind: Option<SocketAddr>,
    pub database_url: String,
    pub redis_url: Option<String>,
    pub qdrant_url: Option<String>,
    pub qdrant_collection: String,
    pub nats_url: Option<String>,
    pub grpc_bind: Option<SocketAddr>,
    pub intelligence_grpc_url: Option<String>,
    pub embedding_provider_url: Option<String>,
    pub embedding_model: String,
    pub embedding_vector_dimensions: usize,
    /// Prepended to user search strings before embedding (asymmetric retrieval models).
    pub embedding_query_prefix: String,
    /// Prepended to block text before embedding (must stay paired with `embedding_query_prefix`).
    pub embedding_document_prefix: String,
    pub embedding_regeneration: EmbeddingRegenerationMode,
    pub cors_origins: Vec<String>,
    pub cookie_secure: bool,
    pub log_filter: String,
    pub mcp_enabled: bool,
    pub mcp_http_path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Environment {
    Dev,
    Prod,
}

impl Default for CoreConfig {
    fn default() -> Self {
        Self {
            environment: Environment::Dev,
            http_bind: SocketAddr::from(([0, 0, 0, 0], 40_000)),
            metrics_bind: Some(SocketAddr::from(([0, 0, 0, 0], 40_001))),
            database_url: "sqlite:data/core.db".to_owned(),
            redis_url: None,
            qdrant_url: None,
            qdrant_collection: "notalking_blocks".to_owned(),
            nats_url: None,
            grpc_bind: None,
            intelligence_grpc_url: None,
            embedding_provider_url: None,
            embedding_model: "text-embedding-3-small".to_owned(),
            embedding_vector_dimensions: 1536,
            embedding_query_prefix: String::new(),
            embedding_document_prefix: String::new(),
            embedding_regeneration: EmbeddingRegenerationMode::AfterQuietPeriodSinceLastPatch {
                idle_ms: 2_000,
            },
            cors_origins: vec![],
            cookie_secure: false,
            log_filter: "info".to_owned(),
            mcp_enabled: true,
            mcp_http_path: "/mcp".to_owned(),
        }
    }
}

#[derive(Parser, Debug)]
#[command(name = "notalking-core")]
#[command(about = "Notalking Core Service")]
struct Cli {
    /// dev | prod
    #[arg(long, env = "CORE_ENV")]
    environment: Option<String>,

    #[arg(long, env = "CORE_HTTP_BIND")]
    http_bind: Option<String>,

    #[arg(long, env = "CORE_METRICS_BIND")]
    metrics_bind: Option<String>,

    #[arg(long, env = "DATABASE_URL")]
    database_url: Option<String>,

    #[arg(long, env = "REDIS_URL")]
    redis_url: Option<String>,

    #[arg(long, env = "QDRANT_URL")]
    qdrant_url: Option<String>,

    #[arg(long, env = "QDRANT_COLLECTION")]
    qdrant_collection: Option<String>,

    #[arg(long, env = "NATS_URL")]
    nats_url: Option<String>,

    #[arg(long, env = "CORE_GRPC_BIND")]
    grpc_bind: Option<String>,

    #[arg(long, env = "INTELLIGENCE_GRPC_URL")]
    intelligence_grpc_url: Option<String>,

    #[arg(long, env = "EMBEDDING_PROVIDER_URL")]
    embedding_provider_url: Option<String>,

    #[arg(long, env = "EMBEDDING_MODEL")]
    embedding_model: Option<String>,

    #[arg(long, env = "CORE_EMBEDDING_VECTOR_DIMENSIONS")]
    embedding_vector_dimensions: Option<usize>,

    /// Prefix for search-query strings (e.g. Nomic: "search_query: ")
    #[arg(long, env = "EMBEDDING_QUERY_PREFIX")]
    embedding_query_prefix: Option<String>,

    /// Prefix for indexed block text (e.g. Nomic: "search_document: ")
    #[arg(long, env = "EMBEDDING_DOCUMENT_PREFIX")]
    embedding_document_prefix: Option<String>,

    /// eager | quiet (default quiet with CORE_EMBEDDING_IDLE_MS)
    #[arg(long, env = "CORE_EMBEDDING_MODE")]
    embedding_mode: Option<String>,

    #[arg(long, env = "CORE_EMBEDDING_IDLE_MS", default_value_t = 2000_u64)]
    embedding_idle_ms: u64,

    /// Comma-separated browser origins for CORS (empty = disable CORS layer)
    #[arg(long, env = "CORE_CORS_ORIGINS")]
    cors_origins: Option<String>,

    #[arg(long, env = "CORE_COOKIE_SECURE")]
    cookie_secure: Option<bool>,

    #[arg(long, env = "RUST_LOG")]
    log_filter: Option<String>,

    #[arg(long, env = "CORE_MCP_ENABLED")]
    mcp_enabled: Option<bool>,

    #[arg(long, env = "CORE_MCP_HTTP_PATH")]
    mcp_http_path: Option<String>,
}

fn parse_socket(s: &str) -> Option<SocketAddr> {
    SocketAddr::from_str(s).ok()
}

fn env_trim(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|v| v.trim().to_owned())
        .filter(|v| !v.is_empty())
}

impl CoreConfig {
    /// Precedence: CLI > environment variables > defaults (SPEC 8).
    pub fn load() -> Self {
        let cli = Cli::parse();
        let default = Self::default();

        let environment = cli
            .environment
            .clone()
            .or_else(|| env_trim("CORE_ENV"))
            .as_deref()
            .map(|s| match s.to_lowercase().as_str() {
                "prod" | "production" => Environment::Prod,
                _ => Environment::Dev,
            })
            .unwrap_or(default.environment);

        let http_bind = cli
            .http_bind
            .clone()
            .or_else(|| env_trim("CORE_HTTP_BIND"))
            .and_then(|s| parse_socket(&s))
            .unwrap_or(default.http_bind);

        let metrics_bind = cli
            .metrics_bind
            .clone()
            .or_else(|| env_trim("CORE_METRICS_BIND"))
            .and_then(|s| {
                if s.eq_ignore_ascii_case("none") || s.is_empty() {
                    None
                } else {
                    parse_socket(&s)
                }
            })
            .or(default.metrics_bind);

        let database_url = cli
            .database_url
            .clone()
            .or_else(|| env_trim("DATABASE_URL"))
            .unwrap_or(default.database_url.clone());

        let redis_url = cli
            .redis_url
            .clone()
            .or_else(|| env_trim("REDIS_URL"))
            .or(default.redis_url.clone());

        let qdrant_url = cli
            .qdrant_url
            .clone()
            .or_else(|| env_trim("QDRANT_URL"))
            .or(default.qdrant_url.clone());

        let qdrant_collection = cli
            .qdrant_collection
            .clone()
            .or_else(|| env_trim("QDRANT_COLLECTION"))
            .unwrap_or(default.qdrant_collection.clone());

        let nats_url = cli
            .nats_url
            .clone()
            .or_else(|| env_trim("NATS_URL"))
            .or(default.nats_url.clone());

        let grpc_bind = cli
            .grpc_bind
            .clone()
            .or_else(|| env_trim("CORE_GRPC_BIND"))
            .and_then(|s| {
                if s.eq_ignore_ascii_case("none") || s.is_empty() {
                    None
                } else {
                    parse_socket(&s)
                }
            })
            .or(default.grpc_bind);

        let intelligence_grpc_url = cli
            .intelligence_grpc_url
            .clone()
            .or_else(|| env_trim("INTELLIGENCE_GRPC_URL"))
            .or(default.intelligence_grpc_url.clone());

        let embedding_provider_url = cli
            .embedding_provider_url
            .clone()
            .or_else(|| env_trim("EMBEDDING_PROVIDER_URL"))
            .or(default.embedding_provider_url.clone());

        let embedding_model = cli
            .embedding_model
            .clone()
            .or_else(|| env_trim("EMBEDDING_MODEL"))
            .unwrap_or(default.embedding_model.clone());

        let embedding_vector_dimensions = cli
            .embedding_vector_dimensions
            .or_else(|| env_trim("CORE_EMBEDDING_VECTOR_DIMENSIONS").and_then(|s| s.parse().ok()))
            .unwrap_or(default.embedding_vector_dimensions);

        let embedding_query_prefix = cli
            .embedding_query_prefix
            .clone()
            .or_else(|| std::env::var("EMBEDDING_QUERY_PREFIX").ok())
            .unwrap_or_default();
        let embedding_document_prefix = cli
            .embedding_document_prefix
            .clone()
            .or_else(|| std::env::var("EMBEDDING_DOCUMENT_PREFIX").ok())
            .unwrap_or_default();

        let embedding_mode = cli
            .embedding_mode
            .clone()
            .or_else(|| env_trim("CORE_EMBEDDING_MODE"))
            .map(|s| s.to_lowercase());

        let idle_ms = cli.embedding_idle_ms;

        let embedding_regeneration = match embedding_mode.as_deref() {
            Some(m) if m == "eager" => EmbeddingRegenerationMode::OnEachBlockPatch,
            Some(m) if m == "quiet" => {
                EmbeddingRegenerationMode::AfterQuietPeriodSinceLastPatch { idle_ms }
            }
            _ => default.embedding_regeneration,
        };

        let cors_origins = cli
            .cors_origins
            .clone()
            .or_else(|| env_trim("CORE_CORS_ORIGINS"))
            .map(|s| {
                s.split(',')
                    .map(|p| p.trim().to_owned())
                    .filter(|p| !p.is_empty())
                    .collect()
            })
            .unwrap_or(default.cors_origins.clone());

        let cookie_secure = cli
            .cookie_secure
            .or_else(|| {
                std::env::var("CORE_COOKIE_SECURE").ok().and_then(|v| {
                    match v.to_lowercase().as_str() {
                        "1" | "true" | "yes" => Some(true),
                        "0" | "false" | "no" => Some(false),
                        _ => None,
                    }
                })
            })
            .unwrap_or(matches!(environment, Environment::Prod));

        let log_filter = cli
            .log_filter
            .clone()
            .or_else(|| env_trim("RUST_LOG"))
            .unwrap_or(default.log_filter.clone());

        let mcp_enabled = cli
            .mcp_enabled
            .or_else(|| {
                std::env::var("CORE_MCP_ENABLED").ok().and_then(|v| {
                    match v.to_lowercase().as_str() {
                        "0" | "false" | "no" => Some(false),
                        "1" | "true" | "yes" => Some(true),
                        _ => None,
                    }
                })
            })
            .unwrap_or(default.mcp_enabled);

        let mcp_http_path = cli
            .mcp_http_path
            .clone()
            .or_else(|| env_trim("CORE_MCP_HTTP_PATH"))
            .unwrap_or(default.mcp_http_path.clone());

        Self {
            environment,
            http_bind,
            metrics_bind,
            database_url,
            redis_url,
            qdrant_url,
            qdrant_collection,
            nats_url,
            grpc_bind,
            intelligence_grpc_url,
            embedding_provider_url,
            embedding_model,
            embedding_vector_dimensions,
            embedding_query_prefix,
            embedding_document_prefix,
            embedding_regeneration,
            cors_origins,
            cookie_secure,
            log_filter,
            mcp_enabled,
            mcp_http_path,
        }
    }
}
