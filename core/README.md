# Core Service

Rust HTTP API for Notalking (Axum). Normative product behavior and boundaries live in [`docs/SPEC.md`](../docs/SPEC.md); this document describes the **current REST surface** as implemented.

Default listen address: **`0.0.0.0:40000`** (`CORE_HTTP_BIND`). There is no global `/api` prefix; routes below are paths on that origin.

---

## Authentication and sessions

- **Mechanism:** HTTP-only cookie **`session_id`** (UUID). Send it on every request that requires a login (`Cookie` header).
- **Registration and login** set `session_id` via **`Set-Cookie`** (`HttpOnly`, `Path=/`, `SameSite=Lax`, **`Max-Age=604800`** seven days). Optional **`Secure`** when `CORE_COOKIE_SECURE` is enabled.
- **Sliding session:** Successful **`authorize_session`** on protected routes extends expiry; responses may include an updated **`Set-Cookie`** with the same policy.
- **Device / location:** Taken from **`User-Agent`** and from **`cf-ipcountry`** or **`x-geo-country`** when present, stored on login/register for session listing.

Browser clients that need cookies must call the API with **credentials** (e.g. `fetch(..., { credentials: 'include' })`) and have the server **CORS** configured (`CORE_CORS_ORIGINS`) if cross-origin.

---

## JSON errors

Failed requests return JSON:

```json
{
  "code": "<stable_machine_code>",
  "message": "human readable",
  "details": null
}
```

`details` may be omitted when empty. Common **`code`** values include:

| Code | Typical HTTP |
|------|----------------|
| `missing_session`, `invalid_session_cookie`, `invalid_credentials`, `session_expired` | 401 |
| `forbidden` | 403 |
| `not_found`, `block_not_found`, `user_not_found`, `session_not_found` | 404 |
| `login_taken`, `session_revoked`, `use_logout` | 409 |
| `invalid_note_id`, `invalid_block_id`, `invalid_after_id`, `invalid_before_id`, `invalid_input`, `invalid_operation` | 400 |
| `embeddings_disabled` | 503 |
| `search_failed` | 502 |
| `rate_limited` | 429 |
| `internal_error`, `corrupt_blocks`, `cookie_write_failed`, `serialize_failed` | 5xx |

---

## Rate limiting

- **Auth routes** under `/auth/*` (except health): **60 requests per minute per client IP** (prefix `auth:{ip}` when Redis or in-memory keying allows).
- **Other protected routes** (notes, search): **60 requests per minute per client IP**.

When `REDIS_URL` is set, limits use Redis; otherwise an in-memory limiter is used (suitable for single-process dev only).

**429** responses include **`Retry-After: 60`** and body `code: "rate_limited"`. If **`ConnectInfo`** (client IP) is unavailable, the key may fall back to `"unknown"` (shared bucket).

---

## Endpoints

### Health

| Method | Path | Description |
|--------|------|-------------|
| **GET** | `/health` | Liveness. **200** empty body. Not cookie-auth protected. |

### Auth (`/auth`)

| Method | Path | Body | Success |
|--------|------|------|---------|
| **POST** | `/auth/register` | `{ "login": string, "password": string }` | **201** + JSON session + `Set-Cookie` |
| **POST** | `/auth/login` | Same as register | **200** + JSON session + `Set-Cookie` |
| **GET** | `/auth/me` | _(cookie)_ | **200** `{ "user_id": string, "session_id": string }` + refreshed cookie -- identity for trusted forwards (e.g. Intelligence) |
| **POST** | `/auth/logout` | _(cookie only)_ | **204** + cleared cookie |
| **GET** | `/auth/sessions` | _(cookie)_ | **200** `{ "sessions": [...] }` + refreshed cookie |
| **DELETE** | `/auth/sessions/{session_id}` | _(cookie)_ | **204** (close another session) + refreshed cookie |
| **DELETE** | `/auth/sessions/others` | _(cookie)_ | **200** `{ "closed_count": number }` + refreshed cookie |

**Session JSON** (register/login response):

```json
{
  "session_id": "<uuid>",
  "user_id": "<uuid>",
  "issued_at": "<rfc3339>",
  "expires_at": "<rfc3339>"
}
```

**Managed session** entries in `sessions` include `device`, `location`, `is_current`, `revoked_at`, etc.

### Notes and blocks

All require **`session_id`** cookie unless noted.

| Method | Path | Body / query | Success |
|--------|------|----------------|---------|
| **GET** | `/notes` | Query: `page` (default 1), `per_page` (default 20, max 100) | **200** paginated list |
| **POST** | `/notes` | `{ "title": string, "body"?: string }` | **201** note + refreshed cookie |
| **DELETE** | `/notes/{note_id}` | — | **204** + refreshed cookie |
| **GET** | `/notes/{note_id}/blocks` | — | **200** `{ "blocks": [ ... ] }` (editor block JSON) + refreshed cookie |
| **POST** | `/notes/{note_id}/blocks` | See **Create block** | **201** block JSON + refreshed cookie |
| **PATCH** | `/notes/{note_id}/blocks/{block_id}` | See **Patch block** | **204** + refreshed cookie |
| **DELETE** | `/notes/{note_id}/blocks/{block_id}` | — | **204** + refreshed cookie |

**Create block** body (discriminated by `type`):

```json
{
  "after_id": "<uuid> | null",
  "type": "text",
  "text": ""
}
```

Only **`text`** blocks are accepted via this API today.

**Patch block** body (discriminated by **`op`**):

| `op` | Fields |
|------|--------|
| `move` | `after_id`, `before_id` (optional UUID strings; mutually exclusive placement per server rules) |
| `insert_text` | `position`, `text`, optional `style` / flattened bold–italic–color |
| `delete_range` | `start`, `end` |
| `delete_at` | `position`, `direction` (`backward` \| `forward`) |
| `enable_formatting` / `disable_formatting` | `start`, `end`, optional style fields |

**List notes** response shape:

```json
{
  "notes": [ { "id", "title", "head_id", "created_at", "updated_at" } ],
  "page": 1,
  "per_page": 20,
  "total": 0,
  "total_pages": 0
}
```

### Semantic search

| Method | Path | Body | Success |
|--------|------|------|---------|
| **POST** | `/search/semantic` | `{ "query": string, "limit"?: number }` (`limit` default 10, max 50) | **200** `{ "hits": [ { "note_id", "block_id", "score" } ] }` + refreshed cookie |

Requires embedding + Qdrant configuration at runtime. If disabled: **503** `embeddings_disabled`. Upstream failures: **502** `search_failed`.

---

## MCP (streamable HTTP)

When **`CORE_MCP_ENABLED`** is true (default), the MCP transport is mounted at **`CORE_MCP_HTTP_PATH`** (default **`/mcp`**). It is a separate HTTP service nested under the same origin; session identity matches REST via the **`session_id`** cookie. Tool listing and protocol details follow the Model Context Protocol; tools mirror note/search operations exposed to HTTP.

---

## Other listeners

| Mechanism | Configuration |
|-----------|----------------|
| **Prometheus metrics** | Optional **`CORE_METRICS_BIND`** (default in code **`0.0.0.0:40001`** when enabled in defaults). |
| **gRPC** | Optional **`CORE_GRPC_BIND`** -- CoreBridge service when set. |

CoreBridge is the private synchronous contract used by Intelligence. It exposes health, current-note metadata, lexical/semantic note search, bounded note content retrieval, and explicit note creation through [`../common/proto/notalking/v1/core.proto`](../common/proto/notalking/v1/core.proto). Search works without Qdrant by matching titles and block text; when embeddings are configured, semantic hits are merged into the same response.

---

## Configuration reference (CLI / env)

Precedence: **CLI flags > environment variables > compiled defaults**. Common variables:

| Variable | Purpose |
|----------|---------|
| `CORE_HTTP_BIND` | HTTP listen address |
| `CORE_METRICS_BIND` | Prometheus scrape bind |
| `DATABASE_URL` | Postgres or SQLite URL |
| `REDIS_URL` | Redis for rate limiting |
| `QDRANT_URL` / `QDRANT_COLLECTION` | Vector search (**gRPC**, default local **`http://127.0.0.1:6334`** -- not the REST port 6333) |
| `NATS_URL` | Optional NATS |
| `CORE_GRPC_BIND` | Optional gRPC server; local Justfile defaults to `127.0.0.1:50051` |
| `EMBEDDING_PROVIDER_URL` / `EMBEDDING_MODEL` / `CORE_EMBEDDING_*` | Embeddings pipeline |
| `EMBEDDING_QUERY_PREFIX` / `EMBEDDING_DOCUMENT_PREFIX` | Optional prefixes for asymmetric retrieval (e.g. Nomic **`search_query: `** / **`search_document: `**); empty for symmetric models like OpenAI |
| `CORE_CORS_ORIGINS` | Comma-separated origins for CORS |
| `CORE_COOKIE_SECURE` | Set `Secure` on cookies |
| `CORE_MCP_ENABLED` / `CORE_MCP_HTTP_PATH` | MCP HTTP mount |

**Local Ollama (OpenAI-compatible):** set `EMBEDDING_PROVIDER_URL` to the `/v1` base (for example `http://127.0.0.1:11434/v1`). No API key is required unless your Ollama install enforces one. For **`nomic-embed-text-v2-moe`**, use **`CORE_EMBEDDING_VECTOR_DIMENSIONS=768`** and the query/document prefixes above so indexed blocks and search queries embed in the same retrieval geometry. If you change model, dimension, or document prefix, use a new **`QDRANT_COLLECTION`** or delete the old one, then **re-save notes** (or otherwise trigger reindexing) so vectors are rebuilt. Core only creates a collection when it is missing.

See **`src/config.rs`** for the full CLI surface.

---

## Local run

From repository root (with Docker Compose): **`just run`**. From **`core/`**: **`just run`** (SQLite under `core/data/`) or **`just run-with-compose`** for Postgres-aligned defaults. See the root [`README.md`](../README.md).
