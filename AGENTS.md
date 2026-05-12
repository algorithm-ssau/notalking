# AGENTS

Instructions for AI coding agents and humans working in this repository. **Product and technical truth** live in `docs/SPEC.md`; this file is the **operating guide** for how to work in the monorepo.

## Read first

1. [`docs/SPEC.md`](docs/SPEC.md) -- normative architecture, data model, APIs, observability, configuration precedence, and service boundaries.
2. [`docs/WRITING.md`](docs/WRITING.md) -- English-only docs, punctuation (` -- `, `<->`), prose vs bullet walls when editing `docs/**`.
3. [`docs/DESIGN.md`](docs/DESIGN.md) -- Web UI design system (typography, color, spacing, motion); required when changing `web/` visuals.
4. [`README.md`](README.md) -- repository map and local commands (may be non-English; **implementation docs and code remain English**).

Planned documents (add when created; until then **SPEC** is authoritative):

- `docs/arch/overview.md` -- concise architecture digest if you want a shorter companion to SPEC.
- `docs/api/contract.md` -- optional REST catalog; **gRPC/protobuf** definitions must still live only in `common/proto/**`.

## Cursor rules and skills

- **Rules** (always-on or as configured in Cursor): [`.agents/rules/`](.agents/rules/) -- includes architecture boundaries for TypeScript, Rust, and Python, comment discipline, and change minimalism.
- **Skills** (load when relevant): [`.agents/skills/`](.agents/skills/) -- e.g. Nuxt/Vue references under `.agents/skills/nuxt` and `.agents/skills/vue`.

## Source of truth (order of authority)

1. [`docs/SPEC.md`](docs/SPEC.md)
2. [`common/proto/**`](common/proto/) -- synchronous service contracts (create this tree when adding protos); **no second handwritten sync contract**.
3. [`core/`](core/), [`web/`](web/), [`intelligence/`](intelligence/) -- application code.
4. [`docs/**`](docs/) -- specifications and design docs that track the code.

## Repository layout

```
notalking/
├── core/           # Rust -- Axum HTTP, auth, notes/blocks, embeddings, MCP (see SPEC)
├── web/            # Nuxt 4 -- browser client
├── intelligence/   # Python -- FastAPI, LLM/V2T (optional at runtime; Core works without it)
├── common/proto/   # Protobuf for gRPC between Core and Intelligence (add when needed)
├── docs/           # SPEC, DESIGN, WRITING, future arch/API docs
└── .agents/        # Agent rules and skills
```

**Intelligence is optional:** Core and the Web editor stay usable without Intelligence; AI/V2T features follow SPEC when that service is down or disabled.

## Run and configuration

- **Web (`web/`):** use **Bun** for installs and scripts (`bun install`, `bun run dev`); see [`web/README.md`](web/README.md) and **SPEC** Section 3.
- **Local runs:** use each package `Justfile` where present (`core/Justfile`, `web/Justfile`). A **repository-root** `Justfile` with `just run` for the full stack is the target described in SPEC -- use it when it exists.
- **Backend configuration precedence** (Core and Intelligence): **CLI flags** override **environment variables**, which override **compiled defaults** -- see SPEC Section 8.

## Rules

- Ignore backward compatibility unless explicitly requested. The product is **pre-launch**; prefer breaking clarity over legacy baggage.
- **Do not add** automated tests (unit, integration, etc.) for now -- they add churn during rapid architecture changes.
- When refactoring, **delete** obsolete code; no graveyards.
- **Update `docs/`** in the same change as any architectural, API, contract, or deployment behavior change.
- Prefer **explicit errors** and **fail-fast** behavior over silent recovery.
- Re-architect freely when it improves clarity or scale.
- Keep **one** protobuf-based sync story under `common/proto/**`; do not duplicate it elsewhere.
- **External systems** (Qdrant, Redis, LLM providers, STT backends) sit **behind adapters** -- see `.agents/rules/architecture.mdc`.
- **Code and technical documentation in English.** Follow [`docs/WRITING.md`](docs/WRITING.md) for doc style.
