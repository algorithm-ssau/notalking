<template>
    <Teleport to="body">
        <Transition name="search-modal">
            <div
                v-if="open"
                class="modal-backdrop search-backdrop"
                role="dialog"
                aria-modal="true"
                aria-labelledby="search-title"
                @keydown.down.prevent="moveSelection(1)"
                @keydown.up.prevent="moveSelection(-1)"
                @keydown.enter.prevent="pickSelected"
                @keydown.escape.prevent="emit('close')"
            >
            <div class="search-scrim" aria-hidden="true" @click="emit('close')" />
            <section class="modal-surface search-modal" @click.stop>
                <h2 id="search-title" class="sr-only">Search notes and blocks</h2>
                <div class="search-input-wrap">
                    <UiAppIcon name="search" :size="20" />
                    <input
                        id="semantic-query"
                        v-model="query"
                        type="search"
                        autocomplete="off"
                        placeholder="Search notes and blocks..."
                        aria-label="Search notes and blocks"
                    />
                    <button class="icon-btn" type="button" aria-label="Close search" @click="emit('close')">
                        <UiAppIcon name="close" :size="17" />
                    </button>
                </div>

                <div class="search-results">
                    <p v-if="searchError" class="error-chip">{{ searchError }}</p>
                    <p v-if="embeddingsHint" class="hint-chip">{{ embeddingsHint }}</p>

                    <template v-if="!trimmedQuery">
                        <p class="group-label">Recent</p>
                        <button
                            v-for="row in recentRows"
                            :key="row.key"
                            type="button"
                            :class="['result-row', { 'is-active': isSelected(row) }]"
                            @mouseenter="selectRow(row)"
                            @click="pick(row)"
                        >
                            <UiAppIcon name="note" :size="16" />
                            <span class="result-main">
                                <strong>{{ row.primary }}</strong>
                                <small>{{ row.secondary }}</small>
                            </span>
                            <time>{{ row.meta }}</time>
                        </button>
                        <p v-if="!recentRows.length" class="empty-state">No recent notes yet.</p>
                    </template>

                    <template v-else>
                        <template v-if="titleRows.length">
                            <p class="group-label">Notes</p>
                            <button
                                v-for="row in titleRows"
                                :key="row.key"
                                type="button"
                                :class="['result-row', { 'is-active': isSelected(row) }]"
                                @mouseenter="selectRow(row)"
                                @click="pick(row)"
                            >
                                <UiAppIcon name="note" :size="16" />
                                <span class="result-main">
                                    <strong>
                                        <template v-for="(part, i) in highlight(row.primary)" :key="`${row.key}-${i}`">
                                            <mark v-if="part.match">{{ part.text }}</mark>
                                            <span v-else>{{ part.text }}</span>
                                        </template>
                                    </strong>
                                    <small>{{ row.secondary }}</small>
                                </span>
                                <time>{{ row.meta }}</time>
                            </button>
                        </template>

                        <template v-if="blockRows.length">
                            <p class="group-label">Blocks</p>
                            <button
                                v-for="row in blockRows"
                                :key="row.key"
                                type="button"
                                :class="['result-row', { 'is-active': isSelected(row) }]"
                                @mouseenter="selectRow(row)"
                                @click="pick(row)"
                            >
                                <UiAppIcon name="block" :size="16" />
                                <span class="result-main">
                                    <strong>{{ row.primary }}</strong>
                                    <small>
                                        <template v-for="(part, i) in highlight(row.secondary)" :key="`${row.key}-s-${i}`">
                                            <mark v-if="part.match">{{ part.text }}</mark>
                                            <span v-else>{{ part.text }}</span>
                                        </template>
                                    </small>
                                </span>
                                <time>{{ row.meta }}</time>
                            </button>
                        </template>

                        <p v-if="searching" class="loading-line">
                            <span />
                            <span />
                            <span />
                        </p>
                        <p v-else-if="showNoResults" class="empty-state">
                            No results for <code>{{ trimmedQuery }}</code>. Try different terms.
                        </p>
                    </template>
                </div>
            </section>
        </div>
        </Transition>
    </Teleport>
</template>

<script setup lang="ts">
import type { NoteResponse, SemanticHitResponse } from "~/types/core";
import { getCoreErrorMessage, isEmbeddingsDisabledError } from "~/utils/coreErrors";

type SearchRow = {
    key: string;
    kind: "note" | "block";
    noteId: string;
    blockId: string | null;
    primary: string;
    secondary: string;
    meta: string;
};

const props = defineProps<{
    open: boolean;
    notes: NoteResponse[];
}>();

const emit = defineEmits<{
    close: [];
    navigate: [payload: { noteId: string; blockId: string | null }];
}>();

const api = useCoreApi();
const query = ref("");
const hits = ref<SemanticHitResponse[]>([]);
const searching = ref(false);
const searched = ref(false);
const searchError = ref("");
const embeddingsHint = ref("");
const selectedIndex = ref(0);
let searchTimer: ReturnType<typeof setTimeout> | null = null;
let searchSeq = 0;

const trimmedQuery = computed(() => query.value.trim());

const sortedNotes = computed(() =>
    [...props.notes].sort(
        (a, b) => new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime(),
    ),
);

const recentRows = computed<SearchRow[]>(() =>
    sortedNotes.value.slice(0, 5).map((note) => ({
        key: `recent-${note.id}`,
        kind: "note",
        noteId: note.id,
        blockId: null,
        primary: note.title || "Untitled note",
        secondary: "Recently updated note",
        meta: relativeTime(note.updated_at),
    })),
);

const titleRows = computed<SearchRow[]>(() => {
    const q = trimmedQuery.value.toLowerCase();
    if (!q) {
        return [];
    }
    return sortedNotes.value
        .filter((note) => (note.title || "Untitled note").toLowerCase().includes(q))
        .slice(0, 6)
        .map((note) => ({
            key: `note-${note.id}`,
            kind: "note",
            noteId: note.id,
            blockId: null,
            primary: note.title || "Untitled note",
            secondary: "Title match",
            meta: relativeTime(note.updated_at),
        }));
});

const noteTitleMap = computed(() => {
    const map = new Map<string, string>();
    for (const note of props.notes) {
        map.set(note.id, note.title || "Untitled note");
    }
    return map;
});

const blockRows = computed<SearchRow[]>(() =>
    hits.value.map((hit, index) => ({
        key: `block-${hit.note_id}-${hit.block_id}-${index}`,
        kind: "block",
        noteId: hit.note_id,
        blockId: hit.block_id,
        primary: "Semantic block match",
        secondary: noteTitleMap.value.get(hit.note_id) ?? "Untitled note",
        meta: `score ${hit.score.toFixed(3)}`,
    })),
);

const selectableRows = computed(() =>
    trimmedQuery.value ? [...titleRows.value, ...blockRows.value] : recentRows.value,
);

const showNoResults = computed(
    () =>
        Boolean(trimmedQuery.value) &&
        searched.value &&
        !searching.value &&
        titleRows.value.length === 0 &&
        blockRows.value.length === 0 &&
        !searchError.value,
);

watch(
    () => props.open,
    (value) => {
        if (value) {
            resetState();
            nextTick(() => document.getElementById("semantic-query")?.focus());
        }
    },
);

watch(trimmedQuery, (value) => {
    selectedIndex.value = 0;
    searchError.value = "";
    embeddingsHint.value = "";
    if (searchTimer) {
        clearTimeout(searchTimer);
        searchTimer = null;
    }
    if (!value) {
        hits.value = [];
        searched.value = false;
        searching.value = false;
        return;
    }
    searchTimer = setTimeout(() => {
        void runSemantic(value);
    }, 260);
});

onUnmounted(() => {
    if (searchTimer) {
        clearTimeout(searchTimer);
    }
});

function resetState() {
    query.value = "";
    hits.value = [];
    selectedIndex.value = 0;
    searched.value = false;
    searching.value = false;
    searchError.value = "";
    embeddingsHint.value = "";
}

async function runSemantic(value: string) {
    const seq = ++searchSeq;
    searching.value = true;
    searched.value = true;
    try {
        const res = await api.semanticSearch({ query: value, limit: 12 });
        if (seq === searchSeq) {
            hits.value = res.hits ?? [];
        }
    } catch (e: unknown) {
        if (seq !== searchSeq) {
            return;
        }
        hits.value = [];
        if (isEmbeddingsDisabledError(e)) {
            embeddingsHint.value =
                "Semantic search is not configured on this Core instance. Title matches still work.";
        } else {
            searchError.value = getCoreErrorMessage(e, "Search failed");
        }
    } finally {
        if (seq === searchSeq) {
            searching.value = false;
        }
    }
}

function isSelected(row: SearchRow): boolean {
    return selectableRows.value[selectedIndex.value]?.key === row.key;
}

function selectRow(row: SearchRow) {
    const index = selectableRows.value.findIndex((candidate) => candidate.key === row.key);
    if (index >= 0) {
        selectedIndex.value = index;
    }
}

function moveSelection(delta: number) {
    const rows = selectableRows.value;
    if (!rows.length) {
        return;
    }
    selectedIndex.value = (selectedIndex.value + delta + rows.length) % rows.length;
}

function pickSelected() {
    const row = selectableRows.value[selectedIndex.value];
    if (row) {
        pick(row);
    }
}

function pick(row: SearchRow) {
    emit("navigate", { noteId: row.noteId, blockId: row.blockId });
    emit("close");
}

function relativeTime(iso: string): string {
    const ms = Date.now() - new Date(iso).getTime();
    if (!Number.isFinite(ms)) {
        return "recent";
    }
    const minutes = Math.max(0, Math.round(ms / 60000));
    if (minutes < 1) return "now";
    if (minutes < 60) return `${minutes}m`;
    const hours = Math.round(minutes / 60);
    if (hours < 24) return `${hours}h`;
    const days = Math.round(hours / 24);
    return `${days}d`;
}

function highlight(text: string): Array<{ text: string; match: boolean }> {
    const q = trimmedQuery.value;
    if (!q) {
        return [{ text, match: false }];
    }
    const lower = text.toLowerCase();
    const needle = q.toLowerCase();
    const index = lower.indexOf(needle);
    if (index < 0) {
        return [{ text, match: false }];
    }
    return [
        { text: text.slice(0, index), match: false },
        { text: text.slice(index, index + q.length), match: true },
        { text: text.slice(index + q.length), match: false },
    ].filter((part) => part.text.length > 0);
}
</script>

<style scoped>
.search-backdrop {
    align-items: flex-start;
    justify-content: center;
    padding: 18vh 16px 16px;
}

.search-scrim {
    position: absolute;
    inset: 0;
}

.search-modal {
    position: relative;
    width: min(520px, 100%);
    min-height: 70px;
    max-height: 560px;
    overflow: hidden;
    border-radius: 12px;
    background: var(--bg-1);
}

.search-input-wrap {
    display: flex;
    align-items: center;
    gap: 10px;
    border-bottom: 1px solid color-mix(in srgb, var(--bg-3) 60%, transparent);
    background: var(--bg-1);
    padding: 12px;
    color: var(--text-muted);
}

.search-input-wrap input {
    min-width: 0;
    flex: 1;
    height: 42px;
    border: 0;
    border-radius: 8px;
    background: transparent;
    color: var(--text-primary);
    outline: none;
    padding: 0 4px;
    font-size: 14px;
}

.search-input-wrap input::placeholder {
    color: var(--text-disabled);
}

.search-results {
    max-height: 468px;
    overflow-y: auto;
    padding: 8px;
    background: var(--bg-base);
}

.group-label {
    margin: 11px 9px 6px;
    color: var(--text-muted);
    font-size: 11px;
    font-weight: 500;
}

.result-row {
    display: grid;
    width: 100%;
    min-height: 40px;
    grid-template-columns: 18px minmax(0, 1fr) auto;
    align-items: center;
    gap: 10px;
    border: 0;
    border-radius: 8px;
    background: transparent;
    color: var(--text-muted);
    padding: 6px 10px;
    text-align: left;
    cursor: pointer;
}

.result-row:hover,
.result-row.is-active {
    background: var(--bg-3);
    color: var(--text-primary);
}

.result-main {
    min-width: 0;
}

.result-main strong,
.result-main small {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.result-main strong {
    color: var(--text-secondary);
    font-size: 13px;
    font-weight: 500;
}

.result-main small,
.result-row time {
    color: var(--text-muted);
    font-size: 11px;
}

mark {
    border-radius: 3px;
    background: rgb(31 127 123 / 0.36);
    color: var(--text-primary);
    padding: 0 1px;
}

.empty-state,
.loading-line {
    margin: 12px 10px;
    color: var(--text-muted);
    font-size: 12px;
}

.empty-state code {
    color: var(--text-secondary);
}

.loading-line {
    display: flex;
    gap: 5px;
}

.loading-line span {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: var(--text-muted);
    animation: pulse 900ms ease-in-out infinite;
}

.loading-line span:nth-child(2) {
    animation-delay: 120ms;
}

.loading-line span:nth-child(3) {
    animation-delay: 240ms;
}

@keyframes pulse {
    0%,
    100% {
        opacity: 0.25;
    }

    50% {
        opacity: 1;
    }
}

@keyframes search-backdrop-enter {
    from {
        opacity: 0;
    }
    to {
        opacity: 1;
    }
}

@keyframes search-backdrop-exit {
    from {
        opacity: 1;
    }
    to {
        opacity: 0;
    }
}

.search-modal-enter-active {
    animation: search-backdrop-enter 250ms var(--ease-out) both;
}

.search-modal-leave-active {
    animation: search-backdrop-exit 250ms var(--ease-out) both;
}
</style>
