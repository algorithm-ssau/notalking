<template>
    <Teleport to="body">
        <div
            v-if="open"
            class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4 transition-opacity"
            role="dialog"
            aria-modal="true"
            aria-labelledby="sessions-title"
            @keydown.escape.prevent="emit('close')"
        >
            <div
                class="absolute inset-0"
                aria-hidden="true"
                @click="emit('close')"
            />
            <div
                class="relative max-h-[85vh] w-full max-w-lg overflow-hidden rounded-lg border border-bg-overlay bg-bg-raised shadow-lg transition-transform"
                @click.stop
            >
                <header
                    class="flex items-center justify-between border-b border-bg-overlay px-4 py-3"
                >
                    <h2 id="sessions-title" class="font-rounded text-lg text-fg-primary">
                        Sessions
                    </h2>
                    <button
                        type="button"
                        class="rounded-md px-2 py-1 text-[14px] leading-6 text-fg-muted hover:bg-bg-overlay hover:text-fg-secondary"
                        @click="emit('close')"
                    >
                        Close
                    </button>
                </header>

                <div class="max-h-[60vh] overflow-y-auto p-4">
                    <p
                        v-if="store.actionError"
                        class="mb-3 rounded-md border border-red/40 bg-red/10 px-3 py-2 text-[14px] leading-6 text-red"
                    >
                        {{ store.actionError }}
                    </p>
                    <p
                        v-if="othersMessage"
                        class="mb-3 text-[14px] leading-6 text-green"
                    >
                        {{ othersMessage }}
                    </p>

                    <p
                        v-if="store.loading"
                        class="text-[14px] leading-6 text-fg-muted"
                    >
                        Loading…
                    </p>
                    <ul
                        v-else-if="store.devices.length"
                        class="flex flex-col gap-2"
                    >
                        <li
                            v-for="s in store.devices"
                            :key="s.session_id"
                            class="rounded-md border border-bg-overlay bg-bg-base px-3 py-2 text-[14px] leading-6"
                        >
                            <div class="flex flex-wrap items-start justify-between gap-2">
                                <div class="min-w-0 flex-1 text-fg-primary">
                                    <span
                                        v-if="s.is_current"
                                        class="mr-2 rounded bg-blue/20 px-1.5 py-0.5 text-[12px] font-medium text-blue"
                                    >
                                        This device
                                    </span>
                                    <span class="break-all text-fg-secondary">{{
                                        s.device || "Unknown device"
                                    }}</span>
                                </div>
                                <button
                                    v-if="!s.is_current"
                                    type="button"
                                    class="shrink-0 rounded-md bg-red/20 px-2 py-1 text-[12px] font-medium text-red hover:bg-red/30"
                                    @click="onRevoke(s.session_id)"
                                >
                                    Revoke
                                </button>
                            </div>
                            <p class="mt-1 text-[12px] text-fg-muted">
                                {{ s.location }} · expires {{ formatIso(s.expires_at) }}
                            </p>
                        </li>
                    </ul>
                    <p v-else class="text-[14px] text-fg-muted">No sessions.</p>
                </div>

                <footer
                    class="flex flex-wrap items-center justify-end gap-2 border-t border-bg-overlay px-4 py-3"
                >
                    <button
                        type="button"
                        class="rounded-md bg-bg-overlay px-3 py-1.5 text-[14px] leading-6 text-fg-secondary hover:bg-bg-float hover:text-fg-primary disabled:opacity-50"
                        :disabled="revokingOthers"
                        @click="onRevokeOthers"
                    >
                        Sign out other devices
                    </button>
                </footer>
            </div>
        </div>
    </Teleport>
</template>

<script setup lang="ts">
const props = defineProps<{
    open: boolean;
}>();

const emit = defineEmits<{
    close: [];
}>();

const store = useSessionStore();
const othersMessage = ref("");
const revokingOthers = ref(false);

watch(
    () => props.open,
    (v) => {
        if (v) {
            othersMessage.value = "";
            void store.load();
        }
    },
);

function formatIso(iso: string): string {
    try {
        return new Date(iso).toLocaleString();
    } catch {
        return iso;
    }
}

async function onRevoke(sessionId: string) {
    othersMessage.value = "";
    await store.revokeOne(sessionId);
}

async function onRevokeOthers() {
    othersMessage.value = "";
    revokingOthers.value = true;
    try {
        const n = await store.revokeOthers();
        if (n != null && n > 0) {
            othersMessage.value = `Signed out ${n} other session(s).`;
        } else if (n === 0) {
            othersMessage.value = "No other sessions to sign out.";
        }
    } finally {
        revokingOthers.value = false;
    }
}
</script>
