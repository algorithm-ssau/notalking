<template>
    <Teleport to="body">
        <div
            v-if="visible && rect"
            class="pointer-events-auto fixed z-50 flex items-center gap-0.5 rounded-lg border border-bg-overlay bg-bg-elevated p-1 shadow-lg"
            :style="bubbleStyle"
            @mousedown.prevent
        >
            <button
                type="button"
                class="rounded px-2 py-1 text-[13px] font-semibold text-fg-primary hover:bg-bg-overlay"
                :class="boldOn ? 'bg-bg-overlay' : ''"
                @click="$emit('bold')"
            >
                B
            </button>
            <button
                type="button"
                class="rounded px-2 py-1 text-[13px] italic text-fg-primary hover:bg-bg-overlay"
                :class="italicOn ? 'bg-bg-overlay' : ''"
                @click="$emit('italic')"
            >
                I
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
    const top = props.rect.top - pad - 36;
    const left = props.rect.left + props.rect.width / 2;
    return {
        top: `${Math.max(8, top)}px`,
        left: `${left}px`,
        transform: "translateX(-50%)",
    };
});
</script>
