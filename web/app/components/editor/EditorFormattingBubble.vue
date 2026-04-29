<template>
    <Teleport to="body">
        <div
            v-if="visible && rect"
            class="format-bubble"
            :style="bubbleStyle"
            @mousedown.prevent
        >
            <button type="button" :class="{ 'is-active': boldOn }" @click="$emit('bold')">
                B
            </button>
            <button type="button" :class="{ 'is-active': italicOn }" @click="$emit('italic')">
                <em>I</em>
            </button>
        </div>
    </Teleport>
</template>

<script setup lang="ts">
const props = defineProps<{
    visible: boolean;
    rect: DOMRect | null;
    boldOn: boolean;
    italicOn: boolean;
}>();

defineEmits<{
    bold: [];
    italic: [];
}>();

const bubbleStyle = computed(() => {
    if (!props.rect) {
        return {};
    }
    const pad = 8;
    const top = props.rect.top - pad - 40;
    const left = props.rect.left + props.rect.width / 2;
    return {
        top: `${Math.max(8, top)}px`,
        left: `${left}px`,
        transform: "translateX(-50%)",
    };
});
</script>

<style scoped>
.format-bubble {
    position: fixed;
    z-index: 80;
    display: flex;
    gap: 4px;
    border: 1px solid var(--bg-3);
    border-radius: var(--r-card);
    background: var(--bg-2);
    padding: 4px;
    box-shadow: 0 16px 44px rgb(0 0 0 / 0.45);
}

.format-bubble button {
    display: grid;
    width: 32px;
    height: 28px;
    place-items: center;
    border: 0;
    border-radius: var(--r-item);
    background: transparent;
    color: var(--text-secondary);
    font-size: 13px;
    font-weight: 700;
    cursor: pointer;
}

.format-bubble button:hover,
.format-bubble button.is-active {
    background: var(--bg-3);
    color: var(--text-primary);
}
</style>
