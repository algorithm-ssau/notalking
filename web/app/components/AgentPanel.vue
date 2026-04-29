<template>
    <section class="agent-panel" aria-label="Agent panel">
        <header class="agent-header">
            <div class="agent-title">
                <span :class="['status-dot', { 'is-offline': offline }]" />
                <span>Agent</span>
            </div>
            <button class="provider-chip" type="button" :disabled="offline">
                {{ offline ? "Offline" : "Provider" }}
                <UiAppIcon name="chevronDown" :size="14" />
            </button>
        </header>

        <div v-if="offline" class="agent-status">
            <UiAppIcon name="warning" :size="16" />
            Intelligence offline -- agent features unavailable
        </div>

        <div class="agent-thread">
            <div v-if="offline" class="agent-empty">
                <div class="agent-empty__icon">
                    <UiAppIcon name="agent" :size="28" />
                </div>
                <h2>Agent unavailable</h2>
                <p>Start or configure the Intelligence service to use provider-backed note assistance.</p>
            </div>
            <template v-else>
                <div class="agent-message ai">Ask a question about your notes and I will cite matching blocks.</div>
                <div class="agent-message user">Find open ideas from this week.</div>
                <div class="agent-message ai">Reading recent note blocks<span class="stream-cursor" /></div>
            </template>
        </div>

        <footer class="agent-input-wrap" :class="{ 'is-disabled': offline }">
            <textarea
                class="textarea agent-input"
                placeholder="Ask about your notes..."
                :disabled="offline"
                rows="2"
            />
            <button class="send-button" type="button" :disabled="offline" aria-label="Send message">
                <UiAppIcon name="send" :size="16" />
            </button>
        </footer>
    </section>
</template>

<script setup lang="ts">
const { offline = true } = defineProps<{
    offline?: boolean;
}>();
</script>

<style scoped>
.agent-panel {
    display: grid;
    grid-template-rows: auto auto minmax(0, 1fr) auto;
    height: 100%;
    background: var(--bg-1);
}

.agent-header {
    display: flex;
    min-height: 48px;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    border-bottom: 1px solid var(--bg-3);
    padding: 8px 12px 8px 16px;
}

.agent-title {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    color: var(--text-secondary);
    font-size: 14px;
    font-weight: 500;
}

.provider-chip {
    display: inline-flex;
    height: 28px;
    align-items: center;
    gap: 6px;
    border: 0;
    border-radius: var(--r-pill);
    background: var(--bg-3);
    color: var(--text-muted);
    padding: 0 10px;
    font-size: 12px;
    cursor: pointer;
}

.agent-status {
    display: flex;
    align-items: center;
    gap: 8px;
    border-bottom: 1px solid var(--bg-4);
    background: var(--bg-3);
    color: var(--text-muted);
    padding: 8px 12px;
    font-size: 12px;
    line-height: 20px;
}

.agent-thread {
    min-height: 0;
    overflow-y: auto;
    padding: 16px 12px;
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
    width: 56px;
    height: 56px;
    place-items: center;
    border: 1px solid var(--bg-3);
    border-radius: 50%;
    background: var(--bg-2);
    color: var(--accent-gold);
}

.agent-empty h2 {
    margin: 16px 0 6px;
    color: var(--text-secondary);
    font-family: var(--font-heading);
    font-size: 18px;
}

.agent-empty p {
    max-width: 220px;
    margin: 0;
    color: var(--text-muted);
    font-size: 13px;
    line-height: 22px;
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
}

.agent-message + .agent-message {
    margin-top: 12px;
}

.agent-message.ai {
    border: 1px solid var(--bg-3);
    background: var(--bg-2);
}

.agent-message.user {
    margin-left: auto;
    background: var(--bg-3);
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
    border-top: 1px solid var(--bg-3);
    padding: 10px 12px;
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
    border-radius: 50%;
    background: var(--accent-gold);
    color: #000;
    cursor: pointer;
}

.send-button:disabled {
    cursor: not-allowed;
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
