set shell := ["zsh", "-cu"]

default:
    @just --list

# Start Postgres, Redis, Qdrant, and NATS, then run Core with URLs wired for local dev.
# Embeddings: requires Ollama on localhost (`ollama serve`) with `ollama pull nomic-embed-text-v2-moe`.
# Core calls OpenAI-compatible POST /v1/embeddings; override EMBEDDING_* / QDRANT_COLLECTION if you change models.
# Nomic-style models need asymmetric prefixes on query vs document text; reindex notes after changing prefixes.
run:
    docker compose up -d
    cd core && \
      RUST_LOG="${RUST_LOG:-info}" \
      DATABASE_URL="${DATABASE_URL:-postgres://notalking:notalking@127.0.0.1:5432/notalking}" \
      REDIS_URL="${REDIS_URL:-redis://127.0.0.1:6379}" \
      QDRANT_URL="${QDRANT_URL:-http://127.0.0.1:6334}" \
      QDRANT_COLLECTION="${QDRANT_COLLECTION:-notalking_embed_ollama}" \
      NATS_URL="${NATS_URL:-nats://127.0.0.1:4222}" \
      EMBEDDING_PROVIDER_URL="${EMBEDDING_PROVIDER_URL:-http://127.0.0.1:11434/v1}" \
      EMBEDDING_MODEL="${EMBEDDING_MODEL:-nomic-embed-text-v2-moe}" \
      CORE_EMBEDDING_VECTOR_DIMENSIONS="${CORE_EMBEDDING_VECTOR_DIMENSIONS:-768}" \
      EMBEDDING_QUERY_PREFIX="${EMBEDDING_QUERY_PREFIX:-search_query: }" \
      EMBEDDING_DOCUMENT_PREFIX="${EMBEDDING_DOCUMENT_PREFIX:-search_document: }" \
      cargo run

stack-down:
    docker compose down
