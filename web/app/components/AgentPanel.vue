<template>
    <section class="agent-panel" aria-label="Assistant panel">
        <header class="agent-header">
            <div class="agent-title">
                <span :class="['status-dot', { 'is-offline': offline }]" />
                <span>Assistant</span>
            </div>
            <button class="provider-chip" type="button" :disabled="offline">
                {{ offline ? "Offline" : "Provider" }}
                <UiAppIcon name="chevronDown" :size="14" />
            </button>
        </header>

        <div v-if="offline" class="agent-status">
            <UiAppIcon name="warning" :size="16" />
            Intelligence offline -- assistant features unavailable
        </div>

        <div class="agent-thread">
            <div v-if="offline" class="agent-empty">
                <div class="agent-empty__icon">
                    <UiAppIcon name="agent" :size="28" />
                </div>
                <h2>Assistant unavailable</h2>
                <p>Start or configure the Intelligence service to use provider-backed note assistance.</p>
            </div>
            <template v-else>
                <div class="agent-message ai">Ask a question about your notes and I will use Notalking content as context.</div>
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
    background: #1b1a18;
}

.agent-header {
    display: flex;
    min-height: 42px;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    border-bottom: 1px solid var(--bg-3);
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

.provider-chip {
    display: inline-flex;
    height: 28px;
    align-items: center;
    gap: 6px;
    border: 0;
    border-radius: var(--r-pill);
    background: #25231f;
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
    background: #22201d;
    color: var(--text-tertiary);
    padding: 8px 12px;
    font-size: 13px;
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
    width: 48px;
    height: 48px;
    place-items: center;
    border: 1px solid var(--bg-3);
    border-radius: 50%;
    background: #22201d;
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
}

.agent-message + .agent-message {
    margin-top: 12px;
}

.agent-message.ai {
    border: 1px solid var(--bg-3);
    background: #22201d;
}

.agent-message.user {
    margin-left: auto;
    background: #2b2925;
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
    background: #22201d;
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
    background: var(--accent-primary);
    color: #071514;
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
