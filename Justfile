set shell := ["zsh", "-cu"]

default:
    @just --list

# Run Core only with the local Compose-backed config.
run-core:
    cd core && just run-with-compose

# Start infra, then run Core, Intelligence, and Web together in one terminal.
# Stops all child processes on Ctrl+C.
run-full:
    docker compose up -d
    trap 'kill 0' INT TERM EXIT; \
    just run-core & \
    just run-intelligence & \
    just run-web & \
    wait

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
      CORE_GRPC_BIND="${CORE_GRPC_BIND:-127.0.0.1:50051}" \
      EMBEDDING_PROVIDER_URL="${EMBEDDING_PROVIDER_URL:-http://127.0.0.1:11434/v1}" \
      EMBEDDING_MODEL="${EMBEDDING_MODEL:-nomic-embed-text-v2-moe}" \
      CORE_EMBEDDING_VECTOR_DIMENSIONS="${CORE_EMBEDDING_VECTOR_DIMENSIONS:-768}" \
      EMBEDDING_QUERY_PREFIX="${EMBEDDING_QUERY_PREFIX:-search_query: }" \
      EMBEDDING_DOCUMENT_PREFIX="${EMBEDDING_DOCUMENT_PREFIX:-search_document: }" \
      cargo run

# Frontend on http://127.0.0.1:3000 with Core and Intelligence proxied via Nuxt middleware.
run-web:
    cd web && \
      NUXT_CORE_API_URL="${NUXT_CORE_API_URL:-http://127.0.0.1:40000}" \
      NUXT_INTELLIGENCE_API_URL="${NUXT_INTELLIGENCE_API_URL:-http://127.0.0.1:41000}" \
      NUXT_PUBLIC_INTELLIGENCE_API_URL="${NUXT_PUBLIC_INTELLIGENCE_API_URL:-/intel}" \
      bun dev --host 127.0.0.1 --port "${PORT:-3000}"

stack-down:
    docker compose down

# Intelligence API (FastAPI). Once: `cd intelligence && python3 -m venv .venv && .venv/bin/pip install -e .`
run-intelligence:
    cd intelligence && \
      export INTELLIGENCE_CORE_HTTP_URL="${INTELLIGENCE_CORE_HTTP_URL:-http://127.0.0.1:40000}" && \
      export INTELLIGENCE_CORE_GRPC_URL="${INTELLIGENCE_CORE_GRPC_URL:-127.0.0.1:50051}" && \
      if [ -x .venv/bin/notalking-intelligence ]; then \
        .venv/bin/notalking-intelligence; \
      elif python3 -c 'import uvicorn' >/dev/null 2>&1; then \
        PYTHONPATH=src python3 -m notalking_intelligence; \
      else \
        echo "Intelligence dependencies are not installed."; \
        echo "Run: just setup-intelligence"; \
        exit 1; \
      fi

# Install Intelligence in a local virtualenv for repeatable development.
setup-intelligence:
    cd intelligence && \
      python3 -m venv .venv && \
      .venv/bin/pip install -e .
