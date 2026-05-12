# Notalking web app (Nuxt)

Nuxt frontend for Notalking. The Web app now follows `docs/DESIGN.md`: a marketing landing page, separate authentication pages, and a full-viewport editor shell with global search and settings surfaces. It talks to the Rust **Core** service over HTTP using a session cookie and the dev proxy at `/core/*`.

Quick start: install dependencies (`bun install`), then run `bun run dev`. Use `bun run build` for a production build.

---

## Application map

| Route or file | Role |
| --- | --- |
| `app/pages/index.vue` | Landing page with Nav, Hero, Features, How it works, Stack, MCP, CTA, and Footer sections. |
| `app/pages/login.vue` | Login card matching the DESIGN auth page layout. |
| `app/pages/register.vue` | Registration card with password-strength UI. Core currently accepts login/password; display name is UI-only until Core adds account profile fields. |
| `app/pages/app.vue` | Main three-column editor shell: note list, editor column, and offline Intelligence agent panel. |
| `app/components/SemanticSearchDialog.vue` | Global `Cmd/Ctrl+K` search modal with recent notes, title matches, and semantic block matches when Core embeddings are configured. |
| `app/components/SettingsModal.vue` | Global settings modal with LLM provider unavailable state, sessions management, and account sign-out. |
| `app/components/AgentPanel.vue` | Agent panel visual state. Intelligence endpoints are not wired yet, so it renders the specified offline state. |
| `app/components/editor/*` | Existing block editor implementation, restyled for the DESIGN editor column. |
| `app/components/ui/*` | Small shared logo and icon primitives used by pages and modals. |
| `app/composables/useCoreApi.ts` | `$fetch` wrapper for Core APIs under `/core/...`. |
| `server/middleware/00-core-proxy.ts` | Proxies `/core/*` to `runtimeConfig.coreApiUrl` so cookies stay on the app origin. |

## Editor notes

- The editor remains block-based, not one giant document.
- Text edits are debounced and serialized before patching Core.
- Selection ranges use Unicode scalar positions to match Core's editor model.
- Reordering uses Core's block move patch operations.
- Intelligence is optional: when it is absent, only the agent/provider UI is disabled; notes and search continue through Core.

## Commands

```bash
bun install
bun run dev
bun run build
bun run preview
```
