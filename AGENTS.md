# AGENTS.md - Notalking

This document describes the architecture, modules, and implementation ideas for the Notalking monorepo. It is intended as a reference for both human contributors and AI agents working on the codebase.

---

## Project Overview

Notalking is a web-based note-taking service with an integrated AI agent. Notes are composed of structured blocks. The agent answers questions in natural language based on the user's own data. Voice input is supported for quick capture.

The system is split into three top-level components that live in a single monorepo:

- **Web App** - Nuxt frontend
- **Core Service** - Rust backend, primary business logic and REST API
- **Intelligence Service** - Python backend, everything AI-related

---

## Repository Structure

```
notalking/
├── core/               # Rust - Core Service
│   ├── crates/
│   │   ├── editor/     # Editor Module crate (standalone library)
│   │   ├── idp/        # Identity Provider crate
│   │   ├── search/     # Search Engine
│   │   └── voice/      # Voice Module
│   └── src/main.rs     # HTTP layer, wires crates together
├── intelligence/       # Python - Intelligence Service
│   ├── agent/          # LLM agent, RAG pipeline
│   ├── embeddings/     # Embedding vector generation
│   └── mcp/            # MCP server (internal tools + external interface)
├── web/                # Nuxt - Web App
├── infra/              # Docker Compose, DB migrations
├── docs/               # Specs and architectural decisions
└── Makefile            # Unified entry point: make dev, make test, etc.
```

---

## Core Service

Written in Rust. Exposes the REST API consumed by the Web App. Internally composed of crates, each responsible for a distinct domain. The `src/main.rs` wires crates together, registers HTTP handlers, and injects repository implementations.

**General principles:**
- Crates define domain types and traits. Concrete implementations (repositories, external clients) live in Core Service and are injected.
- All mutations to notes and blocks happen through Core Service - Intelligence Service never writes directly to the database.
- Block content is encrypted before writing to the database. Decryption happens on read, transparently to the caller.

### Editor Module (`crates/editor`)

A standalone Rust crate. Can be developed and tested without any running infrastructure.

A note is an ordered sequence of blocks. Each block has a unique UUID, a type, optional generic metadata, and content. Supported block types: `Text`, `OrderedListItem`, `UnorderedListItem`, `Image`, `Video`.

**Block ordering** uses a doubly linked list: each block stores `prev_id` and `next_id`. This avoids fractional position fields and eliminates reindexing. Insertion between two blocks updates exactly two records in a single transaction.

**List nesting** is limited to depth 0–2 (three levels). Each list item stores a `parent_id` pointing to its parent list item, enabling tree reconstruction without recursive SQL.

**TextBlock** stores text as an ordered list of chunks. Each chunk has a string and a uniform style (bold, italic, color). Adjacent chunks with identical styles are always merged. The module exposes an API for inserting and deleting text, applying and removing formatting, and querying formatting status over a range.

**Metadata** is a Generic type parameter on `Block<M>`. The caller decides what to put there - margins, paddings, custom annotations. Default is `()`.

**Use cases:**
- Insert a new paragraph block after an existing one → update two `next_id`/`prev_id` fields, create the block
- Apply bold to a selection → `enable_formatting(start, end, Style { bold: Some(true), .. })`
- Move a block to a different position → remove from current position, insert at target, all in one transaction
- Load all blocks of a note → fetch by `note_id`, reconstruct order by walking the linked list starting from the block with `prev_id = NULL`

### IdP (`crates/idp`)

Handles identity and access. Combines Auth and User concerns.

Exposes domain types (`User`, `Claims`, `RefreshToken`) and traits (`UserRepository`, `TokenRepository`). Business logic - password hashing with argon2, JWT generation and validation, refresh token rotation, logout blacklisting via Redis - lives entirely in the crate. Repository implementations are in Core Service and injected at startup.

**Use cases:**
- Register → hash password, persist user, return access + refresh tokens
- Login → verify password, issue tokens
- Refresh → validate refresh token, rotate it, issue new access token
- Logout → add access token to Redis blacklist, revoke refresh token
- Protected route → validate JWT signature and expiry, check blacklist

### Voice Module (`crates/voice`)

Receives a raw audio byte stream from the client, forwards it to the Whisper API, and returns the transcribed text. The result is handed to the Editor Module to create a new text block in the target note.

Kept in Rust (not Python) because it is a straightforward HTTP call to an external API - there is nothing Python-specific about it, and keeping it in Core avoids an extra network hop.

**Use cases:**
- User clicks "record" → audio streamed to `POST /voice/transcribe` → transcription returned → block created in note

### Search Engine (`crates/search`)

Provides two search modes: full-text search via PostgreSQL and semantic search via Qdrant.

The crate does not create embedding vectors - that is the responsibility of Intelligence Service. When a note changes, Intelligence Service generates updated vectors and sends them to Core Service, which stores them in Qdrant. Search Engine queries Qdrant at search time.

Full-text search uses PostgreSQL `tsvector`. Semantic search queries Qdrant with the vector generated from the user's query string (requested from Intelligence Service on demand).

**Use cases:**
- `GET /search?q=dentist&mode=fulltext` → PostgreSQL tsvector match
- `GET /search?q=when is my appointment&mode=semantic` → query vector fetched from Intelligence, Qdrant nearest-neighbor search
- Note updated → Intelligence Service notified asynchronously → new vectors computed → stored in Qdrant

### Integration Module (`crates/integration`) - optional

Not part of MVP. Intended to connect external services (email, calendar, messengers) and surface their data through the agent. Outlined with dashed borders on the architecture diagram to indicate it is non-critical.

---

## Intelligence Service

Written in Python. Handles all AI-related concerns. Can be developed and tested independently from Core Service - use mock responses for notes and search during early development once the API contract is agreed upon.

### Agent (`intelligence/agent`)

Implements the user-facing AI dialog. On each message:
1. Retrieve relevant note chunks via RAG pipeline (semantic search results from Core)
2. Build a prompt with retrieved context
3. Call the configured LLM provider
4. Return the response, optionally with references to source notes

Supports multiple LLM providers: Claude (Anthropic API), GitHub Copilot (primary free option for development), and others. Provider is configurable per deployment or per user. When the agent is disabled, the system functions as a plain note editor with no AI features.

**Use cases:**
- "What do I have planned for today?" → search notes for task-related content → summarize and list
- "When is my doctor's appointment?" → find relevant note → extract date and return
- "Summarize my notes on distributed systems" → retrieve all notes tagged or semantically related → produce summary

### Embeddings (`intelligence/embeddings`)

Generates vector representations of note content using an embedding model. Called asynchronously after a note is modified. Sends resulting vectors to Core Service for storage in Qdrant.

Also generates vectors for user search queries on demand (called by Core Service's Search Engine during semantic search).

**Use cases:**
- Note block updated → embedding job enqueued → vectors computed → POST to Core Service → stored in Qdrant
- User sends semantic search query → Core requests query vector from Intelligence → Qdrant search executed

### MCP (`intelligence/mcp`)

The MCP server serves a dual purpose.

**Internal:** provides the agent with tools to interact with the full system - read notes, create notes, search, manage user session. This is how the agent can act on the system, not just answer questions.

**External:** exposes Notalking as an MCP-compatible tool source. Any external agent (Claude Desktop, Cursor, or any MCP client) can connect and use note-related tools directly, without going through the Notalking web interface.

**Use cases:**
- Agent receives "create a note about today's meeting" → calls `create_note` MCP tool → Core Service persists it
- Agent receives "log me out" → calls `logout` MCP tool → session terminated
- External Claude Desktop instance connects to Notalking MCP server → user queries notes from Claude Desktop without opening the web app

---

## Data Storage

| Store | Purpose |
|---|---|
| PostgreSQL | Users, notes, blocks, refresh tokens |
| Qdrant | Embedding vectors for semantic search |
| Redis | JWT blacklist (revoked tokens on logout) |

Block content and note data are encrypted with a per-user key before being written to PostgreSQL. The encryption layer is transparent to all modules above Core Service.

---

## REST API Summary

All endpoints except `/auth/*` require a valid JWT in the `Authorization` header.

| Endpoint | Methods | Notes |
|---|---|---|
| `/auth/register` | POST | |
| `/auth/login` | POST | |
| `/auth/refresh` | POST | |
| `/auth/logout` | POST | |
| `/users/me` | GET, PATCH | |
| `/notes` | GET, POST | |
| `/notes/:id` | GET, PATCH, DELETE | |
| `/notes/:id/blocks` | GET, POST | `after_id` in body for position |
| `/blocks/:id` | PATCH, DELETE | |
| `/blocks/:id/move` | PATCH | `after_id` in body |
| `/search` | GET | `?q=...&mode=fulltext\|semantic` |
| `/voice/transcribe` | POST | multipart audio |
| `/agent/chat` | POST | SSE for streaming responses |
| `/agent/chat/:session_id` | GET | fetch session history |

---

## Development Notes

- The `editor` crate has no network dependencies - run its tests without any infrastructure.
- The `idp` crate uses trait-based repositories - mock them in tests, no database needed.
- Intelligence Service starts independently with mocked Core API responses. Switch to real Core once `POST /notes` and `GET /notes/:id/blocks` are stable.
- `make dev` starts the full stack via Docker Compose.
- `make test` runs all test suites across Rust crates and Python modules.
- Migrations live in `infra/migrations/` and run automatically on `make dev`.
