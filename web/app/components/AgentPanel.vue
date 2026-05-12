<template>
    <section class="agent-panel" aria-label="Assistant panel">
        <header class="agent-header">
            <div class="agent-title">
                <span :class="['status-dot', { 'is-offline': offline }]" />
                <span>Assistant</span>
            </div>

            <div ref="providerMenuRef" class="provider-menu">
                <button
                    class="provider-chip"
                    type="button"
                    :disabled="offline"
                    :aria-expanded="providerMenuOpen ? 'true' : 'false'"
                    aria-haspopup="menu"
                    @click="toggleProviderMenu"
                >
                    {{ providerLabel }}
                    <UiAppIcon name="chevronDown" :size="14" />
                </button>

                <Transition name="provider-menu">
                    <div v-if="providerMenuOpen && !offline" class="provider-dropdown" role="menu">
                        <button
                            type="button"
                            :class="['provider-option', { 'is-active': !selectedProviderId }]"
                            @click="selectProvider('')"
                        >
                            <span>Default Ollama</span>
                            <small>Use the default Intelligence endpoint.</small>
                        </button>
                        <button
                            v-for="provider in providers"
                            :key="provider.id"
                            type="button"
                            :class="['provider-option', { 'is-active': provider.id === selectedProviderId }]"
                            @click="selectProvider(provider.id)"
                        >
                            <span>{{ provider.display_name || providerKindLabel(provider.kind) }}</span>
                            <small>{{ providerKindLabel(provider.kind) }} · {{ providerModelLabel(provider) }}</small>
                        </button>
                        <p v-if="providers.length === 0" class="provider-dropdown-empty">
                            No saved providers yet. Add them in Settings.
                        </p>
                    </div>
                </Transition>
            </div>
        </header>

        <div v-if="offline" class="agent-status">
            <UiAppIcon name="warning" :size="16" />
            Intelligence offline -- assistant features unavailable
        </div>
        <div v-else-if="streamError" class="agent-status">
            <UiAppIcon name="warning" :size="16" />
            {{ streamError }}
        </div>

        <div ref="threadRef" class="agent-thread">
            <div v-if="offline" class="agent-empty">
                <div class="agent-empty__icon">
                    <UiAppIcon name="agent" :size="28" />
                </div>
                <h2>Assistant unavailable</h2>
                <p>Start or configure the Intelligence service to use provider-backed note assistance.</p>
            </div>
            <template v-else>
                <div v-if="turns.length === 0" class="agent-message ai">
                    Ask a question about your notes and I will use Notalking content as context.
                </div>

                <div v-for="(turn, index) in turns" :key="index" class="agent-turn">
                    <div class="agent-message user">
                        {{ turn.userText }}
                    </div>

                    <section v-if="turn.toolCalls.length" class="tool-trace" aria-label="MCP calls">
                        <div class="tool-trace__header">
                            <span>MCP calls</span>
                            <small>{{ pendingToolCount(turn) ? `${pendingToolCount(turn)} running` : "Complete" }}</small>
                        </div>
                        <div class="tool-trace__list">
                            <article
                                v-for="tool in turn.toolCalls"
                                :key="tool.callId"
                                :class="['tool-row', `is-${tool.phase}`]"
                            >
                                <div class="tool-row__meta">
                                    <strong>{{ tool.label }}</strong>
                                    <span>{{ toolPhaseLabel(tool.phase) }}</span>
                                </div>
                                <p>{{ tool.message }}</p>
                            </article>
                        </div>
                    </section>

                    <div v-if="turn.assistantText || turn.pending" class="agent-message ai">
                        {{ turn.assistantText }}
                        <span v-if="turn.pending" class="stream-cursor" />
                    </div>
                </div>
            </template>
        </div>

        <footer class="agent-input-wrap" :class="{ 'is-disabled': offline }">
            <textarea
                v-model="draft"
                class="textarea agent-input"
                placeholder="Ask about your notes..."
                :disabled="offline || busy"
                rows="2"
                @keydown.enter.exact.prevent="send"
            />
            <button class="send-button" type="button" :disabled="offline || busy" aria-label="Send message" @click="send">
                <UiAppIcon name="send" :size="16" />
            </button>
        </footer>
    </section>
</template>

<script setup lang="ts">
import type {
    IntelChatMessage,
    IntelProvider,
    IntelStreamEvent,
    IntelToolEvent,
} from "~/composables/useIntelChat";
import {
    ASSISTANT_PREFERENCES_CHANGED_EVENT,
    readSelectedIntelProviderId,
    readSuperPrompt,
    writeSelectedIntelProviderId,
} from "~/composables/useIntelChat";

type AgentToolCall = {
    callId: string;
    label: string;
    message: string;
    phase: "start" | "done" | "error";
};

type AgentTurn = {
    userText: string;
    assistantText: string;
    pending: boolean;
    toolCalls: AgentToolCall[];
};

const props = withDefaults(defineProps<{
    offline?: boolean;
    noteId?: string;
}>(), {
    offline: true,
    noteId: "",
});

const emit = defineEmits<{
    noteCreated: [payload: { noteId: string; title: string }];
}>();

const intelApi = useIntelApi();
const draft = ref("");
const busy = ref(false);
const streamError = ref("");
const turns = ref<AgentTurn[]>([]);
const providers = ref<IntelProvider[]>([]);
const selectedProviderId = ref("");
const providerMenuOpen = ref(false);
const providerMenuRef = ref<HTMLElement | null>(null);
const threadRef = ref<HTMLElement | null>(null);

const providerLabel = computed(() => {
    if (props.offline) {
        return "Offline";
    }
    const selected = providers.value.find((provider) => provider.id === selectedProviderId.value);
    return selected?.display_name || "Default Ollama";
});

watch(
    () => props.offline,
    async (offline) => {
        if (offline) {
            providerMenuOpen.value = false;
            providers.value = [];
            return;
        }
        await loadProviders();
    },
    { immediate: true },
);

watch(
    turns,
    async () => {
        await nextTick();
        const node = threadRef.value;
        if (!node) {
            return;
        }
        node.scrollTop = node.scrollHeight;
    },
    { deep: true },
);

onMounted(() => {
    if (!import.meta.client) {
        return;
    }
    selectedProviderId.value = readSelectedIntelProviderId();
    window.addEventListener("notalking:intel-providers-changed", onProvidersChanged);
    window.addEventListener(ASSISTANT_PREFERENCES_CHANGED_EVENT, onAssistantPreferencesChanged as EventListener);
    window.addEventListener("pointerdown", onWindowPointerDown);
});

onUnmounted(() => {
    if (!import.meta.client) {
        return;
    }
    window.removeEventListener("notalking:intel-providers-changed", onProvidersChanged);
    window.removeEventListener(ASSISTANT_PREFERENCES_CHANGED_EVENT, onAssistantPreferencesChanged as EventListener);
    window.removeEventListener("pointerdown", onWindowPointerDown);
});

async function loadProviders() {
    try {
        providers.value = await intelApi.fetchIntelProviders();
        if (selectedProviderId.value && !providers.value.some((provider) => provider.id === selectedProviderId.value)) {
            selectedProviderId.value = "";
            writeSelectedIntelProviderId("");
        }
    } catch (error: unknown) {
        providers.value = [];
        streamError.value = error instanceof Error ? error.message : "Could not load providers";
    }
}

function onProvidersChanged() {
    void loadProviders();
}

function onAssistantPreferencesChanged() {
    selectedProviderId.value = readSelectedIntelProviderId();
}

function onWindowPointerDown(event: Event) {
    if (!providerMenuOpen.value) {
        return;
    }
    const target = event.target;
    if (!(target instanceof Node)) {
        return;
    }
    if (!providerMenuRef.value?.contains(target)) {
        providerMenuOpen.value = false;
    }
}

function toggleProviderMenu() {
    if (props.offline) {
        return;
    }
    providerMenuOpen.value = !providerMenuOpen.value;
}

function selectProvider(providerId: string) {
    selectedProviderId.value = providerId;
    writeSelectedIntelProviderId(providerId);
    providerMenuOpen.value = false;
}

function providerKindLabel(kind: string): string {
    if (kind === "github_models") {
        return "GitHub Models";
    }
    if (kind === "github_copilot") {
        return "GitHub Copilot";
    }
    if (kind === "ollama") {
        return "Ollama";
    }
    return kind;
}

function providerModelLabel(provider: IntelProvider): string {
    const model = provider.config?.model;
    if (typeof model === "string" && model.trim()) {
        return model;
    }
    return provider.kind === "ollama" ? "default Ollama model" : "default model";
}

function superPromptValue(): string | undefined {
    const value = readSuperPrompt().trim();
    return value || undefined;
}

function currentTurn(): AgentTurn | undefined {
    return turns.value[turns.value.length - 1];
}

function flattenTurnsWith(userText: string): IntelChatMessage[] {
    const history = turns.value.flatMap((turn): IntelChatMessage[] => {
        const messages: IntelChatMessage[] = [{ role: "user", content: turn.userText }];
        if (turn.assistantText.trim()) {
            messages.push({ role: "assistant", content: turn.assistantText });
        }
        return messages;
    });
    history.push({ role: "user", content: userText });
    return history;
}

function toolLabel(event: IntelToolEvent): string {
    const raw = event.mcp_method || event.name || "core_bridge";
    return raw
        .split("/")
        .map((part) => part.trim().replace(/_/g, " "))
        .join(" / ");
}

function toolMessage(event: IntelToolEvent): string {
    if (event.message?.trim()) {
        return event.message.trim();
    }
    if (event.phase === "start") {
        return `Calling ${toolLabel(event)}.`;
    }
    if (event.phase === "done") {
        return `${toolLabel(event)} completed.`;
    }
    return `${toolLabel(event)} failed.`;
}

function upsertToolCall(event: IntelToolEvent) {
    const turn = currentTurn();
    if (!turn) {
        return;
    }
    const callId = event.call_id || `${event.name}-${turn.toolCalls.length}`;
    const payload: AgentToolCall = {
        callId,
        label: toolLabel(event),
        message: toolMessage(event),
        phase: event.phase,
    };
    const index = turn.toolCalls.findIndex((tool) => tool.callId === callId);
    if (index >= 0) {
        turn.toolCalls[index] = payload;
        return;
    }
    turn.toolCalls.push(payload);
}

function pendingToolCount(turn: AgentTurn): number {
    return turn.toolCalls.filter((tool) => tool.phase === "start").length;
}

function toolPhaseLabel(phase: AgentToolCall["phase"]): string {
    if (phase === "done") {
        return "Done";
    }
    if (phase === "error") {
        return "Failed";
    }
    return "Running";
}

function applyStreamEvent(event: IntelStreamEvent) {
    const turn = currentTurn();

    if (event.type === "tool") {
        upsertToolCall(event);
        return;
    }
    if (event.type === "token") {
        if (!turn) {
            return;
        }
        turn.assistantText += event.text;
        return;
    }
    if (event.type === "done") {
        if (turn) {
            turn.pending = false;
        }
        return;
    }
    if (event.type === "error") {
        if (turn) {
            turn.pending = false;
        }
        streamError.value = event.message || "Assistant request failed";
        return;
    }
    if (event.type === "action" && event.action === "note_created" && event.note_id) {
        emit("noteCreated", {
            noteId: event.note_id,
            title: event.title || "Agent note",
        });
    }
}

async function send() {
    const text = draft.value.trim();
    if (!text || props.offline || busy.value) {
        return;
    }

    streamError.value = "";
    const outboundMessages = flattenTurnsWith(text);
    turns.value.push({
        userText: text,
        assistantText: "",
        pending: true,
        toolCalls: [],
    });
    draft.value = "";
    busy.value = true;
    providerMenuOpen.value = false;

    try {
        await intelApi.streamIntelChat(
            {
                messages: outboundMessages,
                note_id: props.noteId || undefined,
                provider_id: selectedProviderId.value || undefined,
                super_prompt: superPromptValue(),
            },
            applyStreamEvent,
        );
    } catch (error: unknown) {
        const turn = currentTurn();
        if (turn) {
            turn.pending = false;
        }
        streamError.value = error instanceof Error ? error.message : "Assistant request failed";
    } finally {
        const turn = currentTurn();
        if (turn) {
            turn.pending = false;
        }
        busy.value = false;
    }
}
</script>

<style scoped>
.agent-panel {
    display: grid;
    min-height: 0;
    grid-template-rows: auto auto minmax(0, 1fr) auto;
    height: 100%;
    background: transparent;
}

.agent-header {
    display: flex;
    min-height: 42px;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    border-bottom: 1px solid color-mix(in srgb, var(--bg-3) 60%, transparent);
    padding: 6px 10px 6px 14px;
}

.agent-title {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    color: var(--text-secondary);
    font-size: 14px;
    font-weight: 500;
    line-height: 24px;
}

.provider-menu {
    position: relative;
}

.provider-chip {
    display: inline-flex;
    min-width: 132px;
    height: 28px;
    align-items: center;
    justify-content: space-between;
    gap: 6px;
    border: 0;
    border-radius: var(--r-item);
    background: var(--bg-2);
    color: var(--text-muted);
    padding: 0 10px;
    font-size: 12px;
    cursor: pointer;
}

.provider-dropdown {
    position: absolute;
    top: calc(100% + 8px);
    right: 0;
    z-index: 5;
    display: grid;
    width: 260px;
    gap: 6px;
    border: 1px solid color-mix(in srgb, var(--bg-4) 75%, transparent);
    border-radius: 12px;
    background: color-mix(in srgb, var(--bg-1) 92%, #000);
    padding: 8px;
    box-shadow: 0 24px 80px rgb(0 0 0 / 0.5);
    backdrop-filter: blur(18px);
}

.provider-option {
    display: grid;
    gap: 2px;
    border: 0;
    border-radius: 10px;
    background: transparent;
    color: var(--text-secondary);
    padding: 10px 12px;
    text-align: left;
    cursor: pointer;
}

.provider-option span {
    font-size: 13px;
    font-weight: 600;
    line-height: 18px;
}

.provider-option small,
.provider-dropdown-empty {
    color: var(--text-muted);
    font-size: 12px;
    line-height: 18px;
}

.provider-option:hover,
.provider-option.is-active {
    background: var(--bg-3);
}

.provider-option.is-active span {
    color: var(--accent-primary);
}

.provider-dropdown-empty {
    margin: 0;
    padding: 8px 12px 4px;
}

.agent-status {
    display: flex;
    align-items: center;
    gap: 8px;
    border-bottom: 1px solid color-mix(in srgb, var(--bg-4) 70%, transparent);
    background: color-mix(in srgb, var(--warning) 8%, var(--bg-2));
    color: var(--text-secondary);
    padding: 8px 12px;
    font-size: 13px;
    line-height: 20px;
}

.agent-thread {
    display: flex;
    min-height: 0;
    flex-direction: column;
    gap: 12px;
    overflow-y: auto;
    padding: 16px 12px;
}

.agent-turn {
    display: grid;
    gap: 12px;
}

.agent-empty {
    display: grid;
    min-height: 100%;
    place-content: center;
    justify-items: center;
    padding: 24px;
    text-align: center;
}

.agent-empty__icon {
    display: grid;
    width: 48px;
    height: 48px;
    place-items: center;
    border: 1px solid color-mix(in srgb, var(--bg-3) 60%, transparent);
    border-radius: 50%;
    background: var(--bg-2);
    color: var(--accent-primary);
}

.agent-empty h2 {
    margin: 16px 0 6px;
    color: var(--text-secondary);
    font-size: 18px;
    font-weight: 500;
    line-height: 24px;
}

.agent-empty p {
    max-width: 220px;
    margin: 0;
    color: var(--text-muted);
    font-size: 14px;
    line-height: 24px;
}

.agent-message {
    width: fit-content;
    max-width: 84%;
    border-radius: 16px;
    padding: 10px 12px;
    color: var(--text-secondary);
    font-size: 14px;
    font-weight: 300;
    line-height: 22px;
    white-space: pre-wrap;
}

.agent-message.ai {
    border: 1px solid color-mix(in srgb, var(--bg-3) 60%, transparent);
    background: var(--bg-2);
}

.agent-message.user {
    margin-left: auto;
    background: color-mix(in srgb, var(--bg-3) 80%, var(--bg-4));
}

.tool-trace {
    display: grid;
    gap: 8px;
    border: 1px solid color-mix(in srgb, var(--bg-4) 70%, transparent);
    border-radius: 14px;
    background: linear-gradient(180deg, color-mix(in srgb, var(--bg-2) 88%, #101010), var(--bg-1));
    padding: 12px;
}

.tool-trace__header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    color: var(--text-secondary);
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
}

.tool-trace__header small {
    color: var(--text-muted);
    font-size: 11px;
    letter-spacing: normal;
    text-transform: none;
}

.tool-trace__list {
    display: grid;
    gap: 8px;
}

.tool-row {
    display: grid;
    gap: 4px;
    border-radius: 10px;
    background: color-mix(in srgb, var(--bg-3) 78%, transparent);
    padding: 10px;
}

.tool-row__meta {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
}

.tool-row__meta strong {
    color: var(--text-primary);
    font-size: 13px;
    font-weight: 600;
    line-height: 18px;
}

.tool-row__meta span {
    color: var(--text-muted);
    font-size: 11px;
    font-weight: 600;
    line-height: 16px;
    text-transform: uppercase;
}

.tool-row p {
    margin: 0;
    color: var(--text-secondary);
    font-size: 12px;
    line-height: 18px;
}

.tool-row.is-start .tool-row__meta span {
    color: var(--warning);
}

.tool-row.is-done .tool-row__meta span {
    color: var(--success);
}

.tool-row.is-error .tool-row__meta span {
    color: var(--danger);
}

.stream-cursor {
    display: inline-block;
    width: 7px;
    height: 16px;
    margin-left: 3px;
    background: var(--text-muted);
    vertical-align: text-bottom;
    animation: pulse 900ms ease-in-out infinite;
}

.agent-input-wrap {
    position: relative;
    min-height: 64px;
    border-top: 1px solid color-mix(in srgb, var(--bg-3) 60%, transparent);
    padding: 10px 12px;
    background: linear-gradient(180deg, color-mix(in srgb, var(--bg-1) 12%, transparent), var(--bg-1));
}

.agent-input-wrap.is-disabled {
    cursor: not-allowed;
    opacity: 0.4;
}

.agent-input {
    min-height: 44px;
    padding-right: 48px;
    background: var(--bg-2);
}

.send-button {
    position: absolute;
    right: 22px;
    bottom: 20px;
    display: grid;
    width: 32px;
    height: 32px;
    place-items: center;
    border: 0;
    border-radius: 8px;
    background: var(--accent-primary);
    color: #071514;
    cursor: pointer;
}

.send-button:disabled {
    cursor: not-allowed;
}

.provider-menu-enter-active,
.provider-menu-leave-active {
    transition:
        opacity 150ms ease,
        transform 150ms ease;
}

.provider-menu-enter-from,
.provider-menu-leave-to {
    opacity: 0;
    transform: translateY(-4px);
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
</style>
