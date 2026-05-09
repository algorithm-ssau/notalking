<template>
    <main class="app-shell" :class="{ 'has-overlay': activeOverlay }">
        <header class="topbar">
            <div class="topbar-breadcrumb">
                <button class="icon-btn mobile-only" type="button" aria-label="Open notes" @click="openPanel('notes')">
                    <UiAppIcon name="menu" :size="18" />
                </button>
                <strong>Notalking</strong>
                <span class="breadcrumb-separator" aria-hidden="true">/</span>
                <span>{{ resolvedNoteTitle || "No note selected" }}</span>
            </div>

            <button class="search-trigger" type="button" title="Search (Ctrl or Cmd+K)" @click="searchOpen = true">
                <UiAppIcon name="search" :size="16" />
                <span>Search Notalking</span>
            </button>

            <div class="topbar-actions">
                <button class="icon-btn" type="button" disabled aria-label="Voice input unavailable">
                    <UiAppIcon name="mic" :size="18" />
                </button>
                <button class="icon-btn" type="button" aria-label="Open settings" @click="settingsOpen = true">
                    <UiAppIcon name="settings" :size="18" />
                </button>
                <div class="user-avatar" aria-hidden="true">N</div>
            </div>
        </header>

        <div v-if="activeOverlay" class="panel-scrim" @click="activeOverlay = null" />

        <section class="workspace">
            <aside :class="['notes-panel', { 'is-collapsed': notesCollapsed, 'is-open': activeOverlay === 'notes' }]">
                <div class="panel-rail" aria-hidden="false">
                    <button class="icon-btn" type="button" aria-label="Toggle notes" @click="toggleNotesPanel">
                        <UiAppIcon name="panelLeft" :size="18" />
                    </button>
                    <button class="icon-btn" type="button" aria-label="New note" @click="createNote">
                        <UiAppIcon name="plus" :size="18" />
                    </button>
                    <button class="icon-btn" type="button" aria-label="Search notes" @click="searchOpen = true">
                        <UiAppIcon name="search" :size="18" />
                    </button>
                </div>

                <div class="panel-content notes-content">
                    <header class="panel-heading">
                        <span>Notes</span>
                        <div>
                            <button class="icon-btn" type="button" aria-label="New note" @click="createNote">
                                <UiAppIcon name="plus" :size="18" />
                            </button>
                            <button class="icon-btn desktop-only" type="button" aria-label="Collapse notes" @click="notesCollapsed = true">
                                <UiAppIcon name="panelLeft" :size="18" />
                            </button>
                            <button class="icon-btn mobile-only" type="button" aria-label="Close notes" @click="activeOverlay = null">
                                <UiAppIcon name="close" :size="18" />
                            </button>
                        </div>
                    </header>

                    <label class="panel-search">
                        <UiAppIcon name="search" :size="16" />
                        <input v-model="noteFilter" type="search" placeholder="Search notes" />
                    </label>

                    <p v-if="loadError" class="panel-error">{{ loadError }}</p>
                    <p v-if="!sessionReady" class="panel-muted">Loading notes...</p>

                    <div v-else-if="filteredNotes.length" class="note-list">
                        <article
                            v-for="note in filteredNotes"
                            :key="note.id"
                            :class="['note-row', { 'is-active': note.id === resolvedNoteId }]"
                        >
                            <button type="button" @click="selectNote(note.id)">
                                <UiAppIcon name="note" :size="16" />
                                <span>{{ note.title || "Untitled note" }}</span>
                                <time>{{ relativeTime(note.updated_at) }}</time>
                            </button>
                            <button class="icon-btn note-trash" type="button" aria-label="Delete note" @click="confirmDeleteNote(note.id)">
                                <UiAppIcon name="trash" :size="15" />
                            </button>
                        </article>
                    </div>

                    <div v-else class="notes-empty">
                        <div><UiAppIcon name="note" :size="28" /></div>
                        <p>No notes yet</p>
                        <button class="btn btn-ghost" type="button" @click="createNote">Create your first note</button>
                    </div>

                    <footer v-if="notesTotalPages > 1" class="notes-pagination">
                        <button class="btn btn-ghost" type="button" :disabled="notesPage <= 1" @click="goPage(notesPage - 1)">
                            Previous
                        </button>
                        <span>{{ notesPage }} / {{ notesTotalPages }}</span>
                        <button class="btn btn-ghost" type="button" :disabled="notesPage >= notesTotalPages" @click="goPage(notesPage + 1)">
                            Next
                        </button>
                    </footer>
                </div>
            </aside>

            <section class="editor-column">
                <div v-if="!sessionReady" class="editor-state">
                    <span class="loader-dot" />
                    Loading workspace...
                </div>
                <div v-else-if="!resolvedNoteId" class="editor-state">
                    <UiAppIcon name="pen" :size="28" />
                    <h1>No note selected</h1>
                    <p>Create a note to start writing.</p>
                    <button class="btn btn-primary" type="button" @click="createNote">New note</button>
                </div>
                <EditorNoteEditor
                    v-else
                    :key="resolvedNoteId"
                    :note-id="resolvedNoteId"
                    :note-title="resolvedNoteTitle"
                    :focus-block-id="focusBlockId"
                    @focus-block-done="focusBlockId = null"
                />
            </section>

            <aside :class="['agent-shell', { 'is-collapsed': agentCollapsed, 'is-open': activeOverlay === 'agent' }]">
                <div class="panel-rail agent-rail" aria-hidden="false">
                    <button class="icon-btn" type="button" aria-label="Toggle assistant" @click="toggleAgentPanel">
                        <UiAppIcon name="panelRight" :size="18" />
                    </button>
                    <button class="icon-btn" type="button" aria-label="Assistant unavailable" @click="openPanel('agent')">
                        <UiAppIcon name="agent" :size="18" />
                    </button>
                </div>
                <div class="panel-content agent-content">
                    <button class="icon-btn mobile-only agent-close" type="button" aria-label="Close assistant" @click="activeOverlay = null">
                        <UiAppIcon name="close" :size="18" />
                    </button>
                    <AgentPanel :offline="true" />
                </div>
            </aside>
        </section>

        <SettingsModal :open="settingsOpen" @close="settingsOpen = false" @logout="logout" />
        <SemanticSearchDialog
            :open="searchOpen"
            :notes="notes"
            @close="searchOpen = false"
            @navigate="onSearchNavigate"
        />
    </main>
</template>

<script setup lang="ts">
import type { NoteResponse } from "~/types/core";
import { getCoreErrorMessage } from "~/utils/coreErrors";

const api = useCoreApi();
const sessionStore = useSessionStore();
const NOTES_COLLAPSED_KEY = "notalking:dashboard-notes-collapsed:v2";
const AGENT_COLLAPSED_KEY = "notalking:dashboard-agent-collapsed:v2";

const sessionReady = ref(false);
const loadError = ref("");
const notes = ref<NoteResponse[]>([]);
const notesPage = ref(1);
const notesPerPage = ref(100);
const notesTotalPages = ref(0);
const notesTotal = ref(0);
const selectedNoteId = ref("");
const noteFilter = ref("");
const searchOpen = ref(false);
const settingsOpen = ref(false);
const focusBlockId = ref<string | null>(null);
const notesCollapsed = ref(false);
const agentCollapsed = ref(false);
const activeOverlay = ref<"notes" | "agent" | null>(null);
const isCompact = ref(false);

const filteredNotes = computed(() => {
    const q = noteFilter.value.trim().toLowerCase();
    if (!q) {
        return notes.value;
    }
    return notes.value.filter((note) => (note.title || "Untitled note").toLowerCase().includes(q));
});

const resolvedNoteId = computed({
    get(): string {
        const id = selectedNoteId.value;
        if (id && notes.value.some((note) => note.id === id)) {
            return id;
        }
        return notes.value[0]?.id ?? "";
    },
    set(value: string) {
        selectedNoteId.value = value;
    },
});

const resolvedNoteTitle = computed(
    () => notes.value.find((note) => note.id === resolvedNoteId.value)?.title ?? "",
);

watch(notesCollapsed, (value) => {
    if (import.meta.client) {
        localStorage.setItem(NOTES_COLLAPSED_KEY, value ? "1" : "0");
    }
});

watch(agentCollapsed, (value) => {
    if (import.meta.client) {
        localStorage.setItem(AGENT_COLLAPSED_KEY, value ? "1" : "0");
    }
});

function viewportDefaults() {
    if (!import.meta.client) {
        return;
    }
    isCompact.value = window.matchMedia("(max-width: 1023px)").matches;
    const notesStored = localStorage.getItem(NOTES_COLLAPSED_KEY);
    const agentStored = localStorage.getItem(AGENT_COLLAPSED_KEY);
    notesCollapsed.value = notesStored === "1" && !isCompact.value;
    agentCollapsed.value = agentStored == null ? window.matchMedia("(max-width: 1279px)").matches : agentStored === "1";
}

function updateCompact() {
    if (!import.meta.client) {
        return;
    }
    isCompact.value = window.matchMedia("(max-width: 1023px)").matches;
    if (!isCompact.value) {
        activeOverlay.value = null;
    }
}

function toggleNotesPanel() {
    if (isCompact.value) {
        openPanel("notes");
        return;
    }
    notesCollapsed.value = !notesCollapsed.value;
}

function toggleAgentPanel() {
    if (isCompact.value) {
        openPanel("agent");
        return;
    }
    agentCollapsed.value = !agentCollapsed.value;
}

function openPanel(panel: "notes" | "agent") {
    activeOverlay.value = activeOverlay.value === panel ? null : panel;
}

function selectNote(noteId: string) {
    selectedNoteId.value = noteId;
    if (isCompact.value) {
        activeOverlay.value = null;
    }
}

function onSearchNavigate(payload: { noteId: string; blockId: string | null }) {
    selectedNoteId.value = payload.noteId;
    focusBlockId.value = payload.blockId;
}

function onSearchHotkey(ev: KeyboardEvent) {
    if ((ev.metaKey || ev.ctrlKey) && ev.key.toLowerCase() === "k") {
        ev.preventDefault();
        searchOpen.value = true;
    }
}

async function refreshNotes(page?: number) {
    loadError.value = "";
    const p = page ?? notesPage.value;
    try {
        const res = await api.listNotes({ page: p, per_page: notesPerPage.value });
        notes.value = res.notes;
        notesPage.value = res.page;
        notesTotalPages.value = res.total_pages;
        notesTotal.value = res.total;

        if (res.notes.length) {
            const stillValid = selectedNoteId.value && res.notes.some((note) => note.id === selectedNoteId.value);
            if (!stillValid) {
                selectedNoteId.value = res.notes[0]?.id ?? "";
            }
        } else {
            selectedNoteId.value = "";
        }
    } catch (e: unknown) {
        const status = (e as { statusCode?: number })?.statusCode;
        if (status === 401) {
            await navigateTo({ path: "/login" });
            return;
        }
        notes.value = [];
        selectedNoteId.value = "";
        loadError.value = getCoreErrorMessage(e, "Could not load notes");
    } finally {
        sessionReady.value = true;
    }
}

async function goPage(page: number) {
    if (page < 1 || (notesTotalPages.value > 0 && page > notesTotalPages.value)) {
        return;
    }
    notesPage.value = page;
    await refreshNotes(page);
}

async function createNote() {
    loadError.value = "";
    try {
        const note = await api.createNote(`Note ${new Date().toLocaleString()}`, "");
        notesPage.value = 1;
        await refreshNotes(1);
        selectedNoteId.value = note.id;
        if (isCompact.value) {
            activeOverlay.value = null;
        }
    } catch (e: unknown) {
        loadError.value = getCoreErrorMessage(e, "Could not create note");
    }
}

function confirmDeleteNote(noteId: string) {
    if (!confirm("Delete this note? This cannot be undone.")) {
        return;
    }
    void deleteNoteById(noteId);
}

async function deleteNoteById(noteId: string) {
    loadError.value = "";
    try {
        await api.deleteNote(noteId);
        if (selectedNoteId.value === noteId) {
            selectedNoteId.value = "";
        }
        await refreshNotes(notesPage.value);
    } catch (e: unknown) {
        loadError.value = getCoreErrorMessage(e, "Could not delete note");
    }
}

async function logout() {
    try {
        await api.logout();
    } catch {
        // Local session state still clears when Core is already unreachable.
    }
    sessionStore.clear();
    settingsOpen.value = false;
    notes.value = [];
    selectedNoteId.value = "";
    await navigateTo({ path: "/login" });
}

function relativeTime(iso: string): string {
    const ms = Date.now() - new Date(iso).getTime();
    if (!Number.isFinite(ms)) {
        return "";
    }
    const minutes = Math.max(0, Math.round(ms / 60000));
    if (minutes < 1) return "now";
    if (minutes < 60) return `${minutes}m`;
    const hours = Math.round(minutes / 60);
    if (hours < 24) return `${hours}h`;
    const days = Math.round(hours / 24);
    return `${days}d`;
}

onMounted(() => {
    viewportDefaults();
    window.addEventListener("resize", updateCompact);
    window.addEventListener("keydown", onSearchHotkey);
    void refreshNotes();
});

onUnmounted(() => {
    window.removeEventListener("resize", updateCompact);
    window.removeEventListener("keydown", onSearchHotkey);
});
</script>

<style scoped>
.app-shell {
    display: grid;
    grid-template-rows: 40px minmax(0, 1fr);
    height: 100vh;
    overflow: hidden;
    background:
        radial-gradient(circle at 50% -12rem, rgb(61 157 149 / 0.13), transparent 34rem),
        var(--bg-base);
    color: var(--text-primary);
}

.topbar {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(220px, 360px) minmax(0, 1fr);
    align-items: center;
    height: 50px;
    border-bottom: 1px solid color-mix(in srgb, var(--bg-3) 50%, transparent);
    background: rgb(23 22 20 / 0.92);
    padding: 0 12px;
    gap: 12px;
    backdrop-filter: blur(16px);
}

.topbar-breadcrumb,
.topbar-actions {
    display: flex;
    min-width: 0;
    align-items: center;
    gap: 8px;
}

.topbar-breadcrumb span {
    overflow: hidden;
    color: var(--text-tertiary);
    font-size: 13px;
    line-height: 24px;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.topbar-breadcrumb strong {
    color: var(--text-primary);
    font-size: 13px;
    font-weight: 600;
    line-height: 24px;
}

.breadcrumb-separator {
    color: var(--text-disabled) !important;
}

.search-trigger {
    display: inline-flex;
    width: min(340px, 100%);
    height: 36px;
    align-items: center;
    justify-content: flex-start;
    gap: 8px;
    border: 1px solid color-mix(in srgb, var(--bg-3) 60%, transparent);
    border-radius: 8px;
    background: var(--bg-3);
    color: var(--text-muted);
    padding: 0 12px;
    font-size: 14px;
    line-height: 24px;
    cursor: pointer;
    justify-self: center;
    transition:
        background-color 150ms ease,
        border-color 150ms ease,
        color 150ms ease;
}

.search-trigger:hover {
    background: color-mix(in srgb, var(--bg-3) 120%, var(--bg-2));
    border-color: color-mix(in srgb, var(--accent-primary) 25%, var(--bg-3));
    color: var(--text-primary);
}

.topbar-actions {
    justify-content: flex-end;
}

.user-avatar {
    display: grid;
    width: 26px;
    height: 26px;
    place-items: center;
    border-radius: 50%;
    background: linear-gradient(135deg, var(--accent-primary), #8bf2ea);
    color: #061817;
    font-size: 12px;
    font-weight: 700;
}

.workspace {
    display: grid;
    min-height: 0;
    grid-template-columns: auto minmax(0, 1fr) auto;
    gap: 10px;
    padding: 10px;
}

.notes-panel,
.agent-shell {
    position: relative;
    display: grid;
    min-height: 0;
    grid-template-columns: minmax(0, 1fr);
    border: 1px solid color-mix(in srgb, var(--bg-3) 60%, transparent);
    border-radius: 12px;
    background: #1b1a18;
    overflow: hidden;
    box-shadow: inset 0 1px 0 rgb(255 255 255 / 0.015);
    transition: width 250ms var(--ease-out);
}

.notes-panel {
    width: 252px;
}

.agent-shell {
    width: 324px;
}

.notes-panel.is-collapsed,
.agent-shell.is-collapsed {
    width: 42px;
    grid-template-columns: 42px 0;
}

.panel-rail {
    display: none;
    min-height: 0;
    border-right: 1px solid color-mix(in srgb, var(--bg-3) 60%, transparent);
    padding: 6px;
    background: #191816;
}

.agent-rail {
    border-right: 0;
    border-left: 1px solid var(--bg-3);
}

.is-collapsed > .panel-rail {
    display: grid;
    align-content: start;
    gap: 4px;
}

.panel-content {
    min-width: 0;
    min-height: 0;
    overflow: hidden;
}

.notes-content {
    display: grid;
    grid-template-rows: auto auto auto minmax(0, 1fr) auto;
    padding: 12px 10px;
}

.panel-heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    min-height: 32px;
    margin-bottom: 8px;
}

.panel-heading span {
    color: var(--text-secondary);
    font-size: 15px;
    font-weight: 600;
    letter-spacing: -0.01em;
    line-height: 24px;
}

.panel-heading div {
    display: flex;
    gap: 2px;
}

.panel-search {
    display: flex;
    height: 32px;
    align-items: center;
    gap: 8px;
    border-radius: var(--r-item);
    background: var(--bg-3);
    color: var(--text-muted);
    padding: 0 10px;
}

.panel-search input {
    min-width: 0;
    flex: 1;
    border: 0;
    background: transparent;
    color: var(--text-primary);
    font-size: 14px;
    line-height: 24px;
    outline: none;
}

.panel-search input::placeholder {
    color: var(--text-disabled);
}

.panel-error,
.panel-muted {
    margin: 12px 0 0;
    color: var(--danger);
    font-size: 13px;
    line-height: 22px;
}

.panel-muted {
    color: var(--text-muted);
}

.note-list {
    min-height: 0;
    overflow-y: auto;
    margin-top: 10px;
    padding-right: 2px;
}

.note-row {
    position: relative;
    display: grid;
    grid-template-columns: minmax(0, 1fr) 0;
    align-items: center;
    border-radius: var(--r-item);
}

.note-row:hover,
.note-row.is-active {
    background: #25231f;
}

.note-row.is-active button:first-child {
    color: var(--accent-primary);
}

.note-row button:first-child {
    display: grid;
    width: 100%;
    min-height: 38px;
    grid-template-columns: 16px minmax(0, 1fr) auto;
    align-items: center;
    gap: 8px;
    border: 0;
    border-radius: var(--r-item);
    background: transparent;
    color: var(--text-muted);
    padding: 6px 8px;
    text-align: left;
    cursor: pointer;
}

.note-row.is-active button:first-child span {
    color: var(--text-primary);
}

.note-row button:first-child span {
    overflow: hidden;
    color: var(--text-secondary);
    font-size: 14px;
    line-height: 24px;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.note-row time {
    color: var(--text-disabled);
    font-size: 11px;
    line-height: 16px;
}

.note-trash {
    position: absolute;
    right: 4px;
    opacity: 0;
}

.note-row:hover .note-trash {
    opacity: 1;
}

.note-trash:hover {
    color: var(--danger);
}

.notes-empty,
.editor-state {
    display: grid;
    place-content: center;
    justify-items: center;
    text-align: center;
}

.notes-empty {
    min-height: 240px;
    color: var(--text-muted);
}

.notes-empty div,
.editor-state > svg {
    display: grid;
    width: 56px;
    height: 56px;
    place-items: center;
    border: 1px solid var(--bg-3);
    border-radius: 50%;
    background: var(--bg-2);
    color: var(--accent-primary);
}

.notes-empty p,
.editor-state p {
    margin: 12px 0;
    color: var(--text-muted);
    font-size: 14px;
}

.notes-empty .btn {
    min-height: 36px;
    padding: 6px 12px;
    font-size: 13px;
}

.notes-pagination {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 6px;
    border-top: 1px solid var(--bg-3);
    padding-top: 10px;
    color: var(--text-muted);
    font-size: 12px;
}

.notes-pagination .btn {
    min-height: 32px;
    padding: 4px 8px;
    font-size: 12px;
}

.editor-column {
    min-width: 0;
    min-height: 0;
    overflow-y: auto;
    border: 1px solid color-mix(in srgb, var(--bg-3) 70%, transparent);
    border-radius: 18px;
    background:
        linear-gradient(180deg, rgb(255 255 255 / 0.015), transparent 140px),
        var(--bg-base);
}

.editor-state {
    min-height: 100%;
    padding: 32px;
}

.editor-state h1 {
    margin: 16px 0 0;
    font-size: 28px;
}

.loader-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--accent-primary);
    animation: breathe 900ms ease-in-out infinite;
}

.agent-content {
    position: relative;
}

.agent-close {
    position: absolute;
    top: 8px;
    right: 8px;
    z-index: 2;
}

.panel-scrim {
    position: fixed;
    inset: 40px 0 0;
    z-index: 30;
    background: rgb(0 0 0 / 0.45);
    backdrop-filter: blur(3px);
}

.mobile-only {
    display: none;
}

@keyframes breathe {
    0%,
    100% {
        opacity: 0.3;
        transform: scale(0.8);
    }

    50% {
        opacity: 1;
        transform: scale(1.25);
    }
}

@media (max-width: 1023px) {
    .workspace {
        grid-template-columns: minmax(0, 1fr);
    }

    .notes-panel,
    .agent-shell,
    .notes-panel.is-collapsed,
    .agent-shell.is-collapsed {
        position: fixed;
        top: 40px;
        bottom: 0;
        z-index: 40;
        width: min(320px, calc(100vw - 24px));
        grid-template-columns: minmax(0, 1fr);
        border-radius: 0 16px 16px 0;
        transition: transform 250ms var(--ease-out);
    }

    .notes-panel {
        left: 0;
        transform: translateX(-105%);
    }

    .agent-shell {
        right: 0;
        border-radius: 16px 0 0 16px;
        transform: translateX(105%);
    }

    .notes-panel.is-open,
    .agent-shell.is-open {
        transform: translateX(0);
    }

    .notes-panel > .panel-rail,
    .agent-shell > .panel-rail,
    .notes-panel.is-collapsed > .panel-rail,
    .agent-shell.is-collapsed > .panel-rail {
        display: none;
    }

    .notes-panel .panel-content,
    .agent-shell .panel-content {
        display: block;
    }

    .mobile-only {
        display: inline-flex;
    }

    .desktop-only {
        display: none;
    }
}

@media (max-width: 768px) {
    .topbar {
        grid-template-columns: auto 40px auto;
    }

    .topbar-breadcrumb span {
        display: none;
    }

    .search-trigger {
        width: 32px;
        padding: 0;
    }

    .search-trigger span {
        display: none;
    }
}
</style>
