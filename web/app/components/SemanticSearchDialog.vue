<template>
    <Teleport to="body">
        <div
            v-if="open"
            class="fixed inset-0 z-50 flex items-start justify-center bg-black/50 p-4 pt-[12vh] transition-opacity sm:pt-[15vh]"
            role="dialog"
            aria-modal="true"
            aria-labelledby="search-title"
            @keydown.escape.prevent="emit('close')"
        >
            <div
                class="absolute inset-0"
                aria-hidden="true"
                @click="emit('close')"
            />
            <div
                class="relative w-full max-w-lg rounded-lg border border-bg-overlay bg-bg-raised shadow-lg"
                @click.stop
            >
                <header
                    class="flex items-center justify-between border-b border-bg-overlay px-4 py-3"
                >
                    <h2 id="search-title" class="font-rounded text-lg text-fg-primary">
                        Semantic search
                    </h2>
                    <button
                        type="button"
                        class="rounded-md px-2 py-1 text-[14px] leading-6 text-fg-muted hover:bg-bg-overlay hover:text-fg-secondary"
                        @click="emit('close')"
                    >
                        Close
                    </button>
                </header>

                <form class="border-b border-bg-overlay p-4" @submit.prevent="runSearch">
                    <label class="sr-only" for="semantic-query">Search query</label>
                    <div class="flex gap-2">
                        <input
                            id="semantic-query"
                            v-model="query"
                            type="search"
                            autocomplete="off"
                            placeholder="Search your notes…"
                            class="min-w-0 flex-1 rounded-md border border-bg-overlay bg-bg-elevated px-3 py-2 text-[14px] leading-6 text-fg-primary outline-none focus:ring-1 focus:ring-blue"
                        />
                        <button
                            type="submit"
                            class="shrink-0 rounded-md bg-blue px-4 py-2 text-[14px] font-medium text-white hover:opacity-90 disabled:opacity-50"
                            :disabled="searching || !query.trim()"
                        >
                            Search
                        </button>
                    </div>
                </form>

                <div class="max-h-[50vh] overflow-y-auto p-4">
                    <p
                        v-if="searchError"
                        class="mb-3 rounded-md border border-red/40 bg-red/10 px-3 py-2 text-[14px] leading-6 text-red"
                    >
                        {{ searchError }}
                    </p>
                    <p
                        v-if="embeddingsHint"
                        class="mb-3 text-[14px] leading-6 text-fg-secondary"
                    >
                        {{ embeddingsHint }}
                    </p>
                    <p
                        v-if="searching"
                        class="text-[14px] text-fg-muted"
                    >
                        Searching…
                    </p>
                    <ul v-else-if="hits.length" class="flex flex-col gap-2">
                        <li v-for="(h, i) in hits" :key="`${h.note_id}-${h.block_id}-${i}`">
                            <button
                                type="button"
                                class="w-full rounded-md border border-bg-overlay bg-bg-base px-3 py-2 text-left text-[14px] leading-6 transition-colors hover:border-blue/40 hover:bg-bg-elevated"
                                @click="pick(h)"
                            >
                                <span class="font-medium text-fg-primary">{{
                                    titleForNote(h.note_id)
                                }}</span>
                                <span class="ml-2 text-fg-muted">
                                    score {{ h.score.toFixed(3) }}
                                </span>
                                <span class="mt-1 block truncate font-mono text-[12px] text-fg-tertiary">
                                    {{ h.note_id }} / {{ h.block_id }}
                                </span>
                            </button>
                        </li>
                    </ul>
                    <p
                        v-else-if="searched && !searchError"
                        class="text-[14px] text-fg-muted"
                    >
                        No results.
                    </p>
                </div>
            </div>
        </div>
    </Teleport>
</template>

<script setup lang="ts">
import type { SemanticHitResponse } from "~/types/core";
import { getCoreErrorMessage, isEmbeddingsDisabledError } from "~/utils/coreErrors";

const props = defineProps<{
    open: boolean;
    noteTitles: Map<string, string> | Record<string, string>;
}>();

const emit = defineEmits<{
    close: [];
    navigate: [payload: { noteId: string; blockId: string }];
}>();

const api = useCoreApi();
const query = ref("");
const hits = ref<SemanticHitResponse[]>([]);
const searching = ref(false);
const searched = ref(false);
const searchError = ref("");
const embeddingsHint = ref("");

watch(
    () => props.open,
    (v) => {
        if (v) {
            searchError.value = "";
            embeddingsHint.value = "";
            hits.value = [];
            searched.value = false;
            nextTick(() => {
                document.getElementById("semantic-query")?.focus();
            });
        }
    },
);

function titleForNote(noteId: string): string {
    const m = props.noteTitles;
    if (m instanceof Map) {
        return m.get(noteId) ?? "Untitled note";
    }
    return m[noteId] ?? "Untitled note";
}

async function runSearch() {
    const q = query.value.trim();
    if (!q) {
        return;
    }
    searching.value = true;
    searched.value = true;
    searchError.value = "";
    embeddingsHint.value = "";
    hits.value = [];
    try {
        const res = await api.semanticSearch({ query: q, limit: 20 });
        hits.value = res.hits ?? [];
    } catch (e: unknown) {
        if (isEmbeddingsDisabledError(e)) {
            embeddingsHint.value =
                "Semantic search is not configured on this Core instance (embeddings disabled).";
            hits.value = [];
        } else {
            searchError.value = getCoreErrorMessage(e, "Search failed");
        }
    } finally {
        searching.value = false;
    }
}

function pick(hit: SemanticHitResponse) {
    emit("navigate", { noteId: hit.note_id, blockId: hit.block_id });
    emit("close");
}
</script>
