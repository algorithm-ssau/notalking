# Intelligence service

FastAPI service for LLM streaming, voice-to-text adapters, and provider configuration -- see [`docs/SPEC.md`](../docs/SPEC.md) (sections 5.3 and 6.2).

## Prerequisites

- Python 3.11+
- Core HTTP reachable from this process (`INTELLIGENCE_CORE_HTTP_URL`) so forwarded browser cookies validate via `GET /auth/me`
- Optional: Core gRPC listener (`CORE_GRPC_BIND` on Core and matching `INTELLIGENCE_CORE_GRPC_URL`) for chat note grounding (`SearchNotes`, `GetNoteContent`) and explicit note creation (`CreateNote`)
- Optional: Ollama at `INTELLIGENCE_OLLAMA_BASE_URL` for chat streaming defaults (default model `deepseek-r1:8b`; override with `INTELLIGENCE_OLLAMA_MODEL`)

## Setup

```bash
cd intelligence
python3 -m venv .venv
source .venv/bin/activate
pip install -e .
```

## Generate gRPC stubs

After editing [`../common/proto/notalking/v1/core.proto`](../common/proto/notalking/v1/core.proto):

```bash
just gen-proto
```

Outputs Python modules under `src/notalking/v1/`.

## Run

```bash
just dev
# or
notalking-intelligence
```

`just dev` sets local Core defaults for development: `INTELLIGENCE_CORE_HTTP_URL=http://127.0.0.1:40000` and `INTELLIGENCE_CORE_GRPC_URL=127.0.0.1:50051`. If you run `notalking-intelligence` directly, set those variables yourself when Core is not using the same defaults.

Key environment variables (CLI flags override env per SPEC §8 — see `notalking_intelligence.config:load_settings`):

| Variable | Purpose |
|----------|---------|
| `INTELLIGENCE_HTTP_BIND` | Listen address (default `0.0.0.0:41000`) |
| `INTELLIGENCE_METRICS_BIND` | Prometheus scrape bind (default `0.0.0.0:41001`; disable with empty string if supported) |
| `INTELLIGENCE_DATABASE_URL` | Async SQLAlchemy URL (default SQLite under `./data/intelligence.db`) |
| `INTELLIGENCE_CORE_HTTP_URL` | Core HTTP origin for `/auth/me` |
| `INTELLIGENCE_CORE_GRPC_URL` | Core `CoreBridge` gRPC address |
| `INTELLIGENCE_CORS_ORIGINS` | Comma-separated browser origins (must match `CORE_CORS_ORIGINS` on Core for cookie calls) |
| `INTELLIGENCE_OLLAMA_BASE_URL` | Default Ollama OpenAI-compatible root |
| `INTELLIGENCE_OLLAMA_MODEL` | Default chat model id |

## REST overview

| Method | Path | Notes |
|--------|------|------|
| GET | `/health` | Liveness; optional Core gRPC probe (omit `INTELLIGENCE_CORE_GRPC_URL` if Core has no gRPC listener) |
| GET | `/metrics` | Prometheus scrape (same port as `INTELLIGENCE_HTTP_BIND`, path `/metrics`) |
| GET | `/providers/catalog` | Supported provider kinds and config fields (no auth) |
| GET | `/providers` | List configured providers (requires session; secrets masked as `***`) |
| POST | `/providers` | Create provider -- kinds: **`ollama`**, **`github_models`** (alias **`github_copilot`**) |
| PATCH | `/providers/{id}` | Update provider |
| DELETE | `/providers/{id}` | Delete provider |
| POST | `/chat/completions/stream` | SSE chat; default provider uses **Ollama** (`INTELLIGENCE_OLLAMA_*`); or pass `provider_id`; optional `super_prompt` is injected as a hidden system instruction; CoreBridge calls emit `tool` events and explicit note-write requests return preview actions |
| POST | `/chat/note-actions/apply` | Apply a previously previewed note create / append / replace / rename action |
| POST | `/chat/cancel/{stream_id}` | Cancel in-flight generation |
| POST | `/v2t/transcribe/stream` | SSE stub transcription |

### Core note grounding

When **`INTELLIGENCE_CORE_GRPC_URL`** points at Core's **`CORE_GRPC_BIND`** listener, chat requests search and read the authenticated user's Core notes before calling the provider. The context is bounded and injected as a system message so the model can answer from note titles, excerpts, block text, note ids, and block ids. CoreBridge search always has a lexical title/block fallback; if Core embeddings are configured, semantic hits can enrich the same results.

While the request is running, `/chat/completions/stream` emits SSE `tool` events for CoreBridge/MCP-equivalent calls: `search_notes`, `get_note_content`, `create_note`, and `update_note`. Events include `phase` (`start`, `done`, `error`), the CoreBridge method, the nearest MCP method name, note ids/titles when available, search hit counts, a structured `request`, a structured `minimal_response`, and error messages. Web uses these events to show which notes were read before provider tokens arrive and to expose hover details for each call.

Explicit note-writing requests now stop at a preview boundary first. `/chat/completions/stream` emits an SSE `action` event with `action: "note_write_preview"` when the user asks to create or modify a note. The browser shows the proposed title/body diff and only calls `/chat/note-actions/apply` after the user confirms. The apply route then uses CoreBridge `CreateNote` or `UpdateNote`, refreshes note metadata, and triggers the same embedding update path as direct Core writes.

The Web Settings modal can store an Agent **super prompt**. It is sent as `super_prompt` on every chat request and injected before provider streaming as a hidden system message. It is user-controlled prompt text, not a server-side secret.

### Providers

**Ollama** (`kind: ollama`): optional `config.base_url` (default `INTELLIGENCE_OLLAMA_BASE_URL`; either the Ollama root or `/v1` base is accepted), `config.model`.

**GitHub Models** (`kind: github_models`): requires `config.token` (fine-grained PAT with **`models: read`**). Optional `config.base_url` (default `https://models.github.ai`), `config.model` (e.g. `openai/gpt-4.1`), `config.api_version` (GitHub API version header).

Authenticated routes expect the Core **`session_id`** cookie forwarded from the browser (`credentials: 'include'`).

If provider or chat requests fail during auth, call **Core directly** from the Intelligence host:

```bash
curl -i http://127.0.0.1:40000/auth/me
```

With no cookie this should return **401**, not **502**. A 502 usually means the process on `40000` is not the current Core HTTP server, an unavailable proxy is involved, or proxy environment variables are intercepting localhost traffic. Intelligence disables `HTTP_PROXY` / `ALL_PROXY` inheritance for Core auth checks.

### Default chat without a saved provider

The Agent UI calls chat with no `provider_id`, which uses **Ollama** at `INTELLIGENCE_OLLAMA_BASE_URL` / `INTELLIGENCE_OLLAMA_MODEL`. Ensure **`ollama serve`** is running and the model is pulled (`ollama pull <model>`).
