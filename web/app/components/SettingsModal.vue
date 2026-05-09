<template>
    <Teleport to="body">
        <Transition name="settings-modal">
            <div
                v-if="open"
                class="modal-backdrop settings-backdrop"
                role="dialog"
                aria-modal="true"
                aria-labelledby="settings-title"
                @keydown.escape.prevent="emit('close')"
            >
            <div class="settings-scrim" aria-hidden="true" @click="emit('close')" />
            <section class="modal-surface settings-modal" @click.stop>
                <button class="icon-btn close-button" type="button" aria-label="Close settings" @click="emit('close')">
                    <UiAppIcon name="close" :size="20" />
                </button>
                <aside class="settings-nav" aria-label="Settings sections">
                    <div class="nav-header">
                        <h2 id="settings-title">Settings</h2>
                    </div>
                    <button
                        v-for="section in sections"
                        :key="section.id"
                        type="button"
                        :class="['list-row', { 'is-active': activeSection === section.id }]"
                        @click="activeSection = section.id"
                    >
                        <UiAppIcon :name="section.icon" :size="16" />
                        <span>{{ section.label }}</span>
                    </button>
                </aside>

                <div class="settings-content">
                    <section v-if="activeSection === 'providers'" class="settings-pane">
                        <div class="pane-heading">
                            <div>
                                <p class="pane-kicker">Intelligence</p>
                                <h3>Assistant providers</h3>
                            </div>
                            <span class="offline-pill">
                                <span class="status-dot is-offline" />
                                Offline
                            </span>
                        </div>

                        <div class="offline-callout">
                            <UiAppIcon name="warning" :size="18" />
                            <div>
                                <strong>Intelligence offline</strong>
                                <p>Provider controls are disabled until the Intelligence service is reachable.</p>
                            </div>
                        </div>

                        <article class="provider-card is-disabled">
                            <div>
                                <h4>Configured providers</h4>
                                <p>No provider data is available from Intelligence.</p>
                            </div>
                            <span class="toggle" aria-hidden="true" />
                        </article>

                        <button class="btn btn-ghost add-provider" type="button" disabled>
                            <UiAppIcon name="plus" :size="16" />
                            Add provider
                        </button>
                    </section>

                    <section v-else-if="activeSection === 'sessions'" class="settings-pane sessions-pane">
                        <div class="pane-heading">
                            <div>
                                <p class="pane-kicker">Core sessions</p>
                                <h3>Active devices</h3>
                            </div>
                            <div class="revoke-wrap">
                                <button
                                    class="btn btn-secondary danger-outline"
                                    type="button"
                                    :disabled="revokingOthers || otherSessionCount === 0"
                                    @click="confirmRevokeOthers = !confirmRevokeOthers"
                                >
                                    Revoke all other sessions
                                </button>
                                <div v-if="confirmRevokeOthers" class="confirm-popover">
                                    <p>Revoke {{ otherSessionCount }} sessions?</p>
                                    <div>
                                        <button class="btn btn-danger" type="button" @click="onRevokeOthers">Confirm</button>
                                        <button class="btn btn-ghost" type="button" @click="confirmRevokeOthers = false">Cancel</button>
                                    </div>
                                </div>
                            </div>
                        </div>

                        <p v-if="store.actionError" class="error-chip">{{ store.actionError }}</p>
                        <p v-if="othersMessage" class="success-chip">{{ othersMessage }}</p>
                        <p v-if="store.loading" class="muted-copy">Loading sessions...</p>

                        <div v-else-if="store.devices.length" class="sessions-list">
                            <article v-for="session in store.devices" :key="session.session_id" class="session-row">
                                <div class="session-main">
                                    <div class="session-title">
                                        <strong>{{ session.device || "Unknown device" }}</strong>
                                        <span v-if="session.is_current" class="this-device">This device</span>
                                    </div>
                                    <p>{{ session.location || "Unknown location" }}</p>
                                    <small>
                                        Created {{ formatIso(session.issued_at) }} · Last active {{ formatIso(session.updated_at) }}
                                    </small>
                                </div>
                                <button
                                    v-if="!session.is_current"
                                    class="btn btn-ghost row-danger"
                                    type="button"
                                    @click="store.revokeOne(session.session_id)"
                                >
                                    Revoke
                                </button>
                            </article>
                        </div>
                        <p v-else class="muted-copy">No active sessions.</p>
                    </section>

                    <section v-else class="settings-pane account-pane">
                        <div class="pane-heading">
                            <div>
                                <p class="pane-kicker">Account</p>
                                <h3>Identity</h3>
                            </div>
                        </div>

                        <div class="account-card">
                            <div class="avatar">N</div>
                            <div>
                                <h4>Notalking user</h4>
                                <p>Session managed by Core</p>
                            </div>
                        </div>

                        <Transition name="signout">
                            <div v-if="confirmSignOut" class="signout-row">
                                <button class="btn btn-ghost" type="button" @click="confirmSignOut = false">Cancel</button>
                                <button class="btn btn-danger" type="button" @click="emit('logout')">Confirm sign out</button>
                            </div>
                            <button v-else class="btn btn-danger signout-button" type="button" @click="confirmSignOut = true">
                                Sign out
                            </button>
                        </Transition>
                    </section>
                </div>
            </section>
        </div>
        </Transition>
    </Teleport>
</template>

<script setup lang="ts">
const props = defineProps<{
    open: boolean;
}>();

const emit = defineEmits<{
    close: [];
    logout: [];
}>();

const store = useSessionStore();
const activeSection = ref<"providers" | "sessions" | "account">("providers");
const confirmRevokeOthers = ref(false);
const confirmSignOut = ref(false);
const revokingOthers = ref(false);
const othersMessage = ref("");

const sections = [
    { id: "providers", label: "Assistant", icon: "agent" },
    { id: "sessions", label: "Sessions", icon: "shield" },
    { id: "account", label: "Account", icon: "user" },
] as const;

const otherSessionCount = computed(() => store.devices.filter((s) => !s.is_current).length);

watch(
    () => props.open,
    (value) => {
        if (value) {
            confirmRevokeOthers.value = false;
            confirmSignOut.value = false;
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

async function onRevokeOthers() {
    othersMessage.value = "";
    revokingOthers.value = true;
    try {
        const n = await store.revokeOthers();
        if (n != null && n > 0) {
            othersMessage.value = `Revoked ${n} other session(s).`;
        } else if (n === 0) {
            othersMessage.value = "No other sessions to revoke.";
        }
        confirmRevokeOthers.value = false;
    } finally {
        revokingOthers.value = false;
    }
}
</script>

<style scoped>
.settings-backdrop {
    align-items: center;
    justify-content: center;
    padding: 24px;
}

.settings-scrim {
    position: absolute;
    inset: 0;
}

.settings-modal {
    position: relative;
    display: grid;
    grid-template-columns: 190px minmax(0, 1fr);
    width: min(750px, 100%);
    height: min(560px, calc(100vh - 50px));
    overflow: hidden;
    border-radius: 12px;
}

.close-button {
    position: absolute;
    top: 14px;
    right: 14px;
    z-index: 2;
}

.nav-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin-bottom: 16px;
}

.settings-nav {
    border-right: 1px solid color-mix(in srgb, var(--bg-3) 60%, transparent);
    background: var(--bg-1);
    padding: 24px 12px;
}

.settings-nav h2 {
    margin: 0 8px 20px;
    font-size: 20px;
    letter-spacing: -0.02em;
}

.settings-nav .list-row {
    width: 100%;
    color: var(--text-secondary);
    font-size: 14px;
}

.settings-content {
    min-width: 0;
    overflow: hidden;
    background: var(--bg-2);
}

.settings-pane {
    height: 100%;
    overflow-y: auto;
    padding: 44px 32px 28px;
}

.pane-heading {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 20px;
    margin-bottom: 28px;
}

.pane-kicker {
    margin: 0 0 4px;
    color: var(--text-muted);
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 0.12em;
    text-transform: uppercase;
}

.pane-heading h3 {
    margin: 0;
    font-size: 24px;
    letter-spacing: -0.02em;
}

.offline-pill,
.this-device {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    border-radius: 20px;
    background: var(--bg-3);
    color: var(--text-muted);
    padding: 5px 10px;
    font-size: 12px;
}

.offline-callout {
    display: flex;
    gap: 12px;
    border: 1px solid color-mix(in srgb, var(--bg-4) 70%, transparent);
    border-radius: var(--r-card);
    background: color-mix(in srgb, var(--warning) 8%, var(--bg-3));
    padding: 16px;
    color: var(--text-secondary);
}

.offline-callout strong {
    color: var(--text-secondary);
    font-size: 14px;
}

.offline-callout p,
.provider-card p,
.muted-copy,
.account-card p {
    margin: 4px 0 0;
    color: var(--text-muted);
    font-size: 14px;
    line-height: 24px;
}

.provider-card {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    margin-top: 16px;
    border: 1px solid var(--bg-4);
    border-radius: var(--r-item);
    background: var(--bg-3);
    padding: 16px;
}

.provider-card.is-disabled {
    opacity: 0.55;
}

.provider-card h4,
.account-card h4 {
    margin: 0;
    color: var(--text-primary);
    font-size: 16px;
    font-weight: 600;
}

.toggle {
    width: 42px;
    height: 24px;
    border-radius: var(--r-pill);
    background: var(--bg-4);
}

.add-provider {
    margin-top: 16px;
}

.danger-outline {
    min-height: 36px;
    border-color: color-mix(in srgb, var(--danger) 40%, var(--bg-4));
    color: var(--danger);
    padding: 6px 12px;
    font-size: 13px;
}

.revoke-wrap {
    position: relative;
}

.confirm-popover {
    position: absolute;
    top: calc(100% + 8px);
    right: 0;
    z-index: 3;
    width: 230px;
    border: 1px solid var(--bg-4);
    border-radius: var(--r-card);
    background: var(--bg-3);
    padding: 12px;
    box-shadow: 0 20px 50px rgb(0 0 0 / 0.45);
}

.confirm-popover p {
    margin: 0 0 12px;
    color: var(--text-secondary);
    font-size: 14px;
}

.confirm-popover div,
.signout-row {
    display: flex;
    gap: 8px;
}

.confirm-popover .btn,
.signout-row .btn {
    min-height: 36px;
    padding: 6px 12px;
    font-size: 13px;
}

.sessions-list {
    display: grid;
    gap: 10px;
}

.session-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    border: 1px solid var(--bg-3);
    border-radius: var(--r-item);
    background: var(--bg-3);
    padding: 12px;
}

.session-main {
    min-width: 0;
}

.session-title {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
}

.session-title strong {
    color: var(--text-primary);
    font-size: 14px;
}

.this-device {
    background: color-mix(in srgb, var(--accent-primary) 16%, transparent);
    color: var(--success);
}

.session-row p {
    margin: 2px 0 0;
    color: var(--text-muted);
    font-size: 14px;
}

.session-row small {
    color: var(--text-disabled);
    font-size: 12px;
}

.row-danger {
    min-height: 34px;
    color: var(--danger);
    padding: 5px 10px;
    font-size: 12px;
}

.account-card {
    display: flex;
    align-items: center;
    gap: 16px;
    margin-bottom: 32px;
    border: 1px solid var(--bg-3);
    border-radius: var(--r-card);
    background: var(--bg-3);
    padding: 16px;
}

.avatar {
    display: grid;
    width: 48px;
    height: 48px;
    place-items: center;
    border-radius: 50%;
    background: linear-gradient(135deg, var(--accent-primary), #6ee7dc);
    color: #0a0a0a;
    font-weight: 700;
}

.signout-button {
    width: 100%;
}

.signout-row {
    width: 100%;
}

.signout-row .btn {
    flex: 1;
    min-height: 48px;
}

.signout-enter-active,
.signout-leave-active {
    transition: 200ms ease;
}

.signout-enter-from,
.signout-leave-to {
    opacity: 0;
    transform: translateY(4px);
}

@media (max-width: 768px) {
    .settings-backdrop {
        align-items: stretch;
        padding: 0;
    }

    .settings-modal {
        grid-template-columns: 1fr;
        width: 100%;
        height: 100%;
        border-radius: 0;
    }

    .settings-nav {
        display: flex;
        overflow-x: auto;
        align-items: center;
        gap: 8px;
        border-right: 0;
        border-bottom: 1px solid var(--bg-3);
        padding: 12px 52px 12px 12px;
    }

    .settings-nav h2 {
        margin: 0 8px 0 0;
    }

    .settings-nav .list-row {
        width: auto;
        white-space: nowrap;
    }

    .settings-pane {
        padding: 44px 24px 24px;
    }

    .pane-heading,
    .session-row {
        align-items: stretch;
        flex-direction: column;
    }

    .revoke-wrap,
    .danger-outline {
        width: 100%;
    }
}

.settings-modal-enter-active {
    animation: modal-enter 250ms var(--ease-out) both;
}

.settings-modal-leave-active {
    animation: modal-exit 250ms var(--ease-out) both;
}

@keyframes modal-exit {
    from {
        opacity: 1;
        transform: translateY(0) scale(1);
    }
    to {
        opacity: 0;
        transform: translateY(-8px) scale(0.98);
    }
}
</style>
