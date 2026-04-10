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
                        class="rounded-md px-3 py-1.5 text-[14px] leading-6 text-fg-muted hover:text-fg-secondary"
                        @click="logout"
                    >
                        Log out
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
                />
            </section>
        </div>
    </div>
</template>

<script setup lang="ts">
import type { NoteSummary } from "~/types/editor";

const api = useCoreApi();
const sessionOk = ref(false);
const authError = ref("");
const loadError = ref("");
const login = ref("demo");
const password = ref("demo");
const notes = ref<NoteSummary[]>([]);
const selectedNoteId = ref("");

/**
 * Keeps the open note in sync with the list: if nothing is selected or the id is stale,
 * the getter falls back to the first note so the editor always mounts when notes exist.
 */
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

async function refreshNotes() {
    loadError.value = "";
    try {
        const { notes: list } = await api.listNotes();
        notes.value = list;
        if (list.length) {
            const current = selectedNoteId.value;
            const stillValid = current && list.some((n) => n.id === current);
            if (!stillValid) {
                selectedNoteId.value = list[0].id;
            }
        } else {
            selectedNoteId.value = "";
        }
        sessionOk.value = true;
    } catch (e: unknown) {
        sessionOk.value = false;
        const err = e as { data?: { error?: string }; message?: string };
        loadError.value =
            err?.data?.error ?? err?.message ?? "Could not load notes";
    }
}

async function doLogin() {
    authError.value = "";
    try {
        await api.login(login.value, password.value);
        await refreshNotes();
    } catch (e: unknown) {
        const err = e as { data?: { error?: string } };
        authError.value = err?.data?.error ?? "Login failed";
    }
}

async function doRegister() {
    authError.value = "";
    try {
        await api.register(login.value, password.value);
        await refreshNotes();
    } catch (e: unknown) {
        const err = e as { data?: { error?: string } };
        authError.value = err?.data?.error ?? "Registration failed";
    }
}

async function createNote() {
    loadError.value = "";
    try {
        const note = await api.createNote(
            `Note ${new Date().toLocaleString()}`,
            "",
        );
        await refreshNotes();
        selectedNoteId.value = note.id;
    } catch (e: unknown) {
        const err = e as { data?: { error?: string } };
        loadError.value = err?.data?.error ?? "Could not create note";
    }
}

async function logout() {
    try {
        await $fetch("/core/auth/logout", {
            method: "POST",
            credentials: "include",
        });
    } catch {
        /* ignore */
    }
    sessionOk.value = false;
    notes.value = [];
    selectedNoteId.value = "";
}

onMounted(() => {
    refreshNotes().catch(() => {
        sessionOk.value = false;
    });
});
</script>
