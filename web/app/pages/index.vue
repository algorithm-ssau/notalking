<template>
    <div class="min-h-screen bg-bg-base p-6 sm:p-8">
        <div class="mx-auto max-w-3xl">
            <h1 class="font-rounded text-2xl text-fg-primary">Notalking</h1>
            <p class="mt-1 text-[14px] leading-6 text-fg-secondary">
                Note editor backed by the Core API (session cookie, proxied at
                <code class="text-fg-tertiary">/core/*</code>).
            </p>

            <section
                v-if="!sessionOk"
                class="mt-8 rounded-lg border border-bg-overlay bg-bg-raised p-6"
            >
                <h2 class="font-rounded text-lg text-fg-primary">Sign in</h2>
                <form
                    class="mt-4 flex flex-col gap-3 sm:flex-row sm:items-end"
                    @submit.prevent="doLogin"
                >
                    <label
                        class="flex flex-1 flex-col gap-1 text-[14px] leading-6 text-fg-secondary"
                    >
                        Login
                        <input
                            v-model="login"
                            type="text"
                            autocomplete="username"
                            class="rounded-md border border-bg-overlay bg-bg-elevated px-3 py-2 text-fg-primary outline-none focus:ring-1 focus:ring-blue"
                        />
                    </label>
                    <label
                        class="flex flex-1 flex-col gap-1 text-[14px] leading-6 text-fg-secondary"
                    >
                        Password
                        <input
                            v-model="password"
                            type="password"
                            autocomplete="current-password"
                            class="rounded-md border border-bg-overlay bg-bg-elevated px-3 py-2 text-fg-primary outline-none focus:ring-1 focus:ring-blue"
                        />
                    </label>
                    <div class="flex gap-2">
                        <button
                            type="submit"
                            class="rounded-md bg-blue px-4 py-2 text-[14px] leading-6 font-medium text-white hover:opacity-90"
                        >
                            Log in
                        </button>
                        <button
                            type="button"
                            class="rounded-md bg-bg-overlay px-4 py-2 text-[14px] leading-6 text-fg-primary hover:bg-bg-float"
                            @click="doRegister"
                        >
                            Register
                        </button>
                    </div>
                </form>
                <p v-if="authError" class="mt-3 text-[14px] leading-6 text-red">
                    {{ authError }}
                </p>
            </section>

            <section v-else class="mt-8">
                <div class="mb-4 flex flex-wrap items-center gap-3">
                    <label class="text-[14px] leading-6 text-fg-secondary">
                        Note
                        <select
                            v-model="resolvedNoteId"
                            class="ml-2 rounded-md border border-bg-overlay bg-bg-elevated px-2 py-1.5 text-[14px] leading-6 text-fg-primary"
                        >
                            <option
                                v-for="n in notes"
                                :key="n.id"
                                :value="n.id"
                            >
                                {{ n.title }}
                            </option>
                        </select>
                    </label>
                    <button
                        type="button"
                        class="rounded-md bg-bg-overlay px-3 py-1.5 text-[14px] leading-6 text-fg-secondary hover:bg-bg-float hover:text-fg-primary"
                        @click="createNote"
                    >
                        New note
                    </button>
                    <button
                        type="button"
                        class="rounded-md bg-red/20 px-3 py-1.5 text-[14px] leading-6 text-red hover:bg-red/30"
                        :disabled="!resolvedNoteId"
                        @click="confirmDeleteNote"
                    >
                        Delete note
                    </button>
                    <button
                        type="button"
                        title="Search (Ctrl or ⌘+K)"
                        class="rounded-md bg-bg-overlay px-3 py-1.5 text-[14px] leading-6 text-fg-secondary hover:bg-bg-float hover:text-fg-primary"
                        @click="searchOpen = true"
                    >
                        Search
                    </button>
                    <button
                        type="button"
                        class="rounded-md bg-bg-overlay px-3 py-1.5 text-[14px] leading-6 text-fg-secondary hover:bg-bg-float hover:text-fg-primary"
                        @click="sessionsOpen = true"
                    >
                        Sessions
                    </button>
                    <button
                        type="button"
                        class="rounded-md px-3 py-1.5 text-[14px] leading-6 text-fg-muted hover:text-fg-secondary"
                        @click="logout"
                    >
                        Log out
                    </button>
                </div>

                <div
                    v-if="notesTotalPages > 1"
                    class="mb-4 flex flex-wrap items-center gap-2 text-[14px] text-fg-secondary"
                >
                    <button
                        type="button"
                        class="rounded-md bg-bg-overlay px-2 py-1 hover:bg-bg-float disabled:opacity-40"
                        :disabled="notesPage <= 1"
                        @click="goPage(notesPage - 1)"
                    >
                        Previous
                    </button>
                    <span>
                        Page {{ notesPage }} of {{ notesTotalPages }} ({{ notesTotal }} notes)
                    </span>
                    <button
                        type="button"
                        class="rounded-md bg-bg-overlay px-2 py-1 hover:bg-bg-float disabled:opacity-40"
                        :disabled="notesPage >= notesTotalPages"
                        @click="goPage(notesPage + 1)"
                    >
                        Next
                    </button>
                </div>

                <p v-if="loadError" class="mb-4 text-[14px] leading-6 text-red">
                    {{ loadError }}
                </p>
                <EditorNoteEditor
                    v-if="resolvedNoteId"
                    :key="resolvedNoteId"
                    :note-id="resolvedNoteId"
                    :note-title="resolvedNoteTitle"
                    :focus-block-id="focusBlockId"
                    @focus-block-done="focusBlockId = null"
                />
            </section>
        </div>

        <SessionsPanel :open="sessionsOpen" @close="sessionsOpen = false" />
        <SemanticSearchDialog
            :open="searchOpen"
            :note-titles="noteTitleMap"
            @close="searchOpen = false"
            @navigate="onSearchNavigate"
        />
    </div>
</template>

<script setup lang="ts">
import type { NoteResponse } from "~/types/core";
import { getCoreErrorMessage } from "~/utils/coreErrors";

const api = useCoreApi();
const sessionStore = useSessionStore();

const sessionOk = ref(false);
const authError = ref("");
const loadError = ref("");
const login = ref("demo");
const password = ref("demo");
const notes = ref<NoteResponse[]>([]);
const notesPage = ref(1);
const notesPerPage = ref(20);
const notesTotalPages = ref(0);
const notesTotal = ref(0);
const selectedNoteId = ref("");
const sessionsOpen = ref(false);
const searchOpen = ref(false);
const focusBlockId = ref<string | null>(null);

const noteTitleMap = computed(() => {
    const m = new Map<string, string>();
    for (const n of notes.value) {
        m.set(n.id, n.title);
    }
    return m;
});

const resolvedNoteId = computed({
    get(): string {
        const id = selectedNoteId.value;
        if (id && notes.value.some((n) => n.id === id)) {
            return id;
        }
        return notes.value[0]?.id ?? "";
    },
    set(v: string) {
        selectedNoteId.value = v;
    },
});

const resolvedNoteTitle = computed(
    () => notes.value.find((n) => n.id === resolvedNoteId.value)?.title ?? "",
);

function onSearchNavigate(payload: { noteId: string; blockId: string }) {
    selectedNoteId.value = payload.noteId;
    focusBlockId.value = payload.blockId;
}

function onSearchHotkey(ev: KeyboardEvent) {
    if ((ev.metaKey || ev.ctrlKey) && ev.key === "k") {
        ev.preventDefault();
        searchOpen.value = true;
    }
}

async function refreshNotes(page?: number) {
    loadError.value = "";
    const p = page ?? notesPage.value;
    try {
        const res = await api.listNotes({
            page: p,
            per_page: notesPerPage.value,
        });
        notes.value = res.notes;
        notesPage.value = res.page;
        notesTotalPages.value = res.total_pages;
        notesTotal.value = res.total;

        const list = res.notes;
        if (list.length) {
            const current = selectedNoteId.value;
            const stillValid = current && list.some((n) => n.id === current);
            if (!stillValid) {
                selectedNoteId.value = list[0]?.id ?? "";
            }
        } else {
            selectedNoteId.value = "";
        }
        sessionOk.value = true;
    } catch (e: unknown) {
        sessionOk.value = false;
        loadError.value = getCoreErrorMessage(e, "Could not load notes");
    }
}

async function goPage(p: number) {
    if (p < 1 || (notesTotalPages.value > 0 && p > notesTotalPages.value)) {
        return;
    }
    notesPage.value = p;
    await refreshNotes(p);
}

async function doLogin() {
    authError.value = "";
    try {
        await api.login(login.value, password.value);
        notesPage.value = 1;
        await refreshNotes(1);
    } catch (e: unknown) {
        authError.value = getCoreErrorMessage(e, "Login failed");
    }
}

async function doRegister() {
    authError.value = "";
    try {
        await api.register(login.value, password.value);
        notesPage.value = 1;
        await refreshNotes(1);
    } catch (e: unknown) {
        authError.value = getCoreErrorMessage(e, "Registration failed");
    }
}

async function createNote() {
    loadError.value = "";
    try {
        const note = await api.createNote(
            `Note ${new Date().toLocaleString()}`,
            "",
        );
        notesPage.value = 1;
        await refreshNotes(1);
        selectedNoteId.value = note.id;
    } catch (e: unknown) {
        loadError.value = getCoreErrorMessage(e, "Could not create note");
    }
}

function confirmDeleteNote() {
    const id = resolvedNoteId.value;
    if (!id) {
        return;
    }
    if (!confirm("Delete this note? This cannot be undone.")) {
        return;
    }
    void deleteNoteById(id);
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
        /* ignore */
    }
    sessionOk.value = false;
    notes.value = [];
    selectedNoteId.value = "";
    sessionStore.clear();
}

onMounted(() => {
    window.addEventListener("keydown", onSearchHotkey);
    refreshNotes().catch(() => {
        sessionOk.value = false;
    });
});

onUnmounted(() => {
    window.removeEventListener("keydown", onSearchHotkey);
});
</script>
