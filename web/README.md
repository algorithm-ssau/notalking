# Notalking web app (Nuxt)

Nuxt frontend for Notalking. The **note editor** loads notes from the Rust **Core** service over HTTP, using a **session cookie** and a dev proxy at `/core/*`.

Quick start: install dependencies (`bun install`), then `bun run dev`. See [Nuxt docs](https://nuxt.com/docs/getting-started/introduction) for build, preview, and deployment.

---

## Note editor — overview

### What it consists of

| Piece | Role |
|--------|------|
| **`app/pages/index.vue`** | Auth (login/register), note list dropdown, toolbar to create notes. Renders the editor when signed in. |
| **`app/components/editor/NoteEditor.vue`** | Auto-imported as **`EditorNoteEditor`** (Nuxt naming). Loads blocks for a note, lists **one Vue component per text block**, handles **HTML5 drag-and-drop** reordering (left handle on each block), drop-slot highlights, **bold/italic** via a floating bubble, and Core API calls for create/move/patch/delete. |
| **`app/components/editor/EditorTextBlock.vue`** | **contenteditable** surface for a single block: renders styled chunks as HTML, **debounced** sync (~300 ms) to Core, IME/composition handling, selection reporting for the formatting bubble. |
| **`app/components/editor/EditorFormattingBubble.vue`** | Anchored **B / I** controls for the current selection range. |
| **`app/types/editor.ts`** | `NoteBlock`, `TextChunk`, `getTextContent` (handles `text` / `Text` payload shapes), **`scalarLen`** and helpers for plain text from chunks. |
| **`app/composables/useCoreApi.ts`** | `fetch` wrapper: credentials, JSON, errors; talks to **`/core/...`** (same origin in dev via proxy). |
| **`server/middleware/00-core-proxy.ts`** | Proxies `/core/*` to `runtimeConfig.coreApiUrl` so the browser keeps cookies on the app origin. |
| **`nuxt.config.ts`** | `runtimeConfig.coreApiUrl` and related wiring. |

### UX and layout (intended behavior)

- **Notion-like** editing: discrete blocks, not one giant document.
- **Typography**: body text **14px / 24px** line height; **Tailwind** for layout and theme tokens (`text-fg-*`, `bg-bg-*`, etc.).
- **Block chrome**: **4–8px**-style padding around the editable area; **left drag handle** (⋮⋮) starts native drag; **drop indicators** (highlight + horizontal line) show where a block will land.
- **Formatting**: select text → **bubble** with bold and italic; ranges are expressed in **Unicode scalar** indices end-to-end with the Core editor crate.

### Data flow and sync

- Blocks come from Core as a **doubly linked list** (`prev_id` / `next_id`). The parent keeps an ordered list of **text** blocks for the UI.
- Local edits are **debounced** before computing a diff and sending **patch** requests; patches are **serialized** with a small queue (`enqueuePatch`) so concurrent updates do not interleave unpredictably.
- A **`dirty`** flag on each text block prevents the block’s **watch** from running a DOM rebuild from props while the user is typing or until the parent has finished reacting to `blocks-updated` (avoids wiping the field or the selection at the wrong time).

### Unicode, whitespace, and the caret (important fixes)

1. **Indices are Unicode scalars, not UTF-8 bytes**  
   JavaScript string length and naive byte-style splitting do not match Rust’s `TextBlock` API. The client uses **`scalarLen`** (`[...s].length`) and the same idea for caret/range math. Core’s text layer uses **character (scalar) indices** so scripts such as **Cyrillic** do not corrupt positions or panic the server.

2. **Spaces must be visible and preserved**  
   Default CSS **collapses** whitespace; **`innerText`** also loses leading/trailing and repeated spaces in ways that break sync. The editor uses **`whitespace-pre-wrap`** (and related classes) and builds plain text with **`collectTextFromEditor`**, walking the DOM (text nodes + `<br>`), not `innerText`.

3. **Caret after debounced sync**  
   Rebuilding the block DOM (`innerHTML` / chunk sync) **destroys the selection**; the browser then places the caret at the **start**. The fix is: read **`getCaretScalarOffset()`** before **`syncDomFromChunks()`**, then after **`nextTick`** (and **`focus()`**), restore with **`setCaretPlainOffset`**, clamped to the new plain-text length so server-normalized text does not produce an invalid offset.

### Integration checklist (things that bit us once)

- **Use `EditorNoteEditor` in templates**, not `NoteEditor`: Nuxt resolves `NoteEditor.vue` under `components/editor/` to the **`EditorNoteEditor`** tag.
- **Note selection**: a **`resolvedNoteId`** (computed get/set) avoids an empty selection when the list loads so the editor is not mounted with **0×0** or no note.
- **Core proxy + cookie**: API calls go to **`/core/...`** on the app host so **Set-Cookie** and **credentials** stay consistent.
- **Move / reorder API**: Core supports ordering patches (e.g. move before/after another block); the UI uses these after a successful drop.

### Known edge cases / follow-ups

- If a **patch fails**, error handling may still resync DOM without the same caret restore path; worth aligning with the success path if users report jumps after errors.
- If the **server normalizes** text differently from the client, caret clamping keeps the index valid but the caret might land **slightly** off the visual character; a second **`nextTick`** or **`requestAnimationFrame`** can be tried if needed.

---

## Setup

```bash
bun install
```

## Development

```bash
bun run dev
```

## Production

```bash
bun run build
bun run preview
```
