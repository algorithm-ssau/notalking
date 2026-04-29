<template>
    <article class="note-editor">
        <header class="editor-titlebar">
            <div>
                <p class="editor-kicker">Current note</p>
                <h1 class="editor-title">{{ noteTitle || "Untitled note" }}</h1>
            </div>
            <button class="btn btn-ghost add-block" type="button" @click="addTextBlock">
                <UiAppIcon name="plus" :size="16" />
                Add block
            </button>
        </header>

        <div class="editor-messages">
            <p v-if="loadError" class="error-chip">{{ loadError }}</p>
            <p v-if="blockActionError" class="error-chip">{{ blockActionError }}</p>
        </div>

        <div class="block-canvas" @dragend="onDragEnd">
            <div v-if="textBlocks.length === 0" class="empty-editor">
                <p>Start writing...</p>
                <button class="btn btn-ghost" type="button" @click="addTextBlock">
                    Create first block
                </button>
            </div>

            <div v-else class="block-list">
                <template v-for="(block, index) in textBlocks" :key="block.id">
                    <div
                        class="drop-slot"
                        @dragover.prevent="onDragOverSlot(index)"
                        @dragleave="onDragLeaveSlot(index)"
                        @drop.prevent="onDrop(index)"
                    >
                        <div class="drop-hitbox" />
                        <div :class="['drop-line', { 'is-active': dragActive && dropSlot === index }]" />
                    </div>

                    <div :class="['block-row', { 'is-dragging': draggingId === block.id }]">
                        <div class="block-shell" @click.self="onBlockClick(block.id)">
                            <EditorTextBlock
                                :note-id="noteId"
                                :block="block"
                                :is-dragging="draggingId === block.id"
                                @blocks-updated="onBlocksUpdated"
                                @format-select="onFormatSelect(block.id, $event)"
                                @format-clear="onFormatClear"
                                @drag-start="onDragStart"
                            />
                        </div>
                        <button
                            type="button"
                            class="icon-btn delete-block"
                            aria-label="Delete block"
                            @click="removeBlock(block.id)"
                        >
                            <UiAppIcon name="trash" :size="15" />
                        </button>
                    </div>
                </template>

                <div
                    class="drop-slot"
                    @dragover.prevent="onDragOverSlot(textBlocks.length)"
                    @dragleave="onDragLeaveSlot(textBlocks.length)"
                    @drop.prevent="onDrop(textBlocks.length)"
                >
                    <div class="drop-hitbox" />
                    <div :class="['drop-line', { 'is-active': dragActive && dropSlot === textBlocks.length }]" />
                </div>
            </div>
        </div>

        <EditorFormattingBubble
            :visible="!!formatting"
            :rect="formatting?.rect ?? null"
            :bold-on="!!formatting?.bold"
            :italic-on="!!formatting?.italic"
            @bold="applyFormatBold"
            @italic="applyFormatItalic"
        />
    </article>
</template>

<script setup lang="ts">
import type { NoteBlock } from "~/types/editor";
import { getTextContent, scalarLen } from "~/types/editor";
import { getCoreErrorMessage } from "~/utils/coreErrors";

const props = withDefaults(
    defineProps<{
        noteId: string;
        noteTitle: string;
        focusBlockId?: string | null;
    }>(),
    { focusBlockId: null },
);

const emit = defineEmits<{
    "focus-block-done": [];
}>();

const api = useCoreApi();
const blocks = ref<NoteBlock[]>([]);
const loadError = ref("");
const blockActionError = ref("");
const draggingId = ref<string | null>(null);
const dropSlot = ref<number | null>(null);
const formatting = ref<{
    blockId: string;
    start: number;
    end: number;
    rect: DOMRect;
    bold: boolean;
    italic: boolean;
} | null>(null);

const dragActive = computed(() => draggingId.value != null);

const textBlocks = computed(() =>
    blocks.value.filter((b) => getTextContent(b.content) !== null),
);

watch(
    () => props.noteId,
    (id) => {
        if (id) {
            loadBlocks();
        }
    },
);

onMounted(() => {
    if (props.noteId) {
        loadBlocks();
    }
});

watch(
    [() => props.focusBlockId, blocks],
    async () => {
        const id = props.focusBlockId;
        if (!id || import.meta.server) {
            return;
        }
        if (!blocks.value.some((b) => b.id === id)) {
            return;
        }
        await nextTick();
        const el = document.querySelector(`[data-block-id="${id}"]`);
        el?.scrollIntoView({ behavior: "smooth", block: "center" });
        emit("focus-block-done");
    },
    { flush: "post" },
);

async function loadBlocks() {
    if (import.meta.server) {
        return;
    }
    loadError.value = "";
    try {
        const res = await api.listBlocks(props.noteId);
        const list = Array.isArray(res?.blocks) ? res.blocks : [];
        blocks.value = list as NoteBlock[];
    } catch (e: unknown) {
        blocks.value = [];
        loadError.value = getCoreErrorMessage(
            e,
            "Could not load note blocks.",
        );
    }
}

function onBlocksUpdated(list: NoteBlock[]) {
    blocks.value = list;
}

async function addTextBlock() {
    blockActionError.value = "";
    const list = textBlocks.value;
    const afterId = list.length ? list[list.length - 1].id : null;
    try {
        await api.createTextBlock(props.noteId, afterId, "");
        await loadBlocks();
    } catch (e: unknown) {
        blockActionError.value = getCoreErrorMessage(
            e,
            "Could not add block.",
        );
    }
}

async function removeBlock(blockId: string) {
    if (!confirm("Delete this block?")) {
        return;
    }
    blockActionError.value = "";
    try {
        await api.deleteBlock(props.noteId, blockId);
        await loadBlocks();
    } catch (e: unknown) {
        blockActionError.value = getCoreErrorMessage(
            e,
            "Could not delete block.",
        );
    }
}

function onDragStart(id: string) {
    draggingId.value = id;
}

function onDragEnd() {
    draggingId.value = null;
    dropSlot.value = null;
}

function onDragOverSlot(slot: number) {
    if (!draggingId.value) {
        return;
    }
    dropSlot.value = slot;
}

function onDragLeaveSlot(slot: number) {
    if (dropSlot.value === slot) {
        dropSlot.value = null;
    }
}

async function onDrop(slotIndex: number) {
    const dragged = draggingId.value;
    if (!dragged) {
        return;
    }
    const list = textBlocks.value;
    if (list.length === 0) {
        onDragEnd();
        return;
    }

    try {
        if (slotIndex === 0) {
            await api.patchBlock(props.noteId, dragged, {
                op: "move",
                before_id: list[0].id,
            });
        } else {
            const prev = list[slotIndex - 1];
            const afterId = prev?.id;
            if (!afterId || afterId === dragged) {
                onDragEnd();
                return;
            }
            await api.patchBlock(props.noteId, dragged, {
                op: "move",
                after_id: afterId,
            });
        }
        await loadBlocks();
    } finally {
        onDragEnd();
    }
}

function selectionStyleInRange(block: NoteBlock, start: number, end: number) {
    const tc = getTextContent(block.content);
    if (!tc) {
        return { bold: false, italic: false };
    }
    let pos = 0;
    let bold = false;
    let italic = false;
    let any = false;
    for (const c of tc.chunks) {
        const next = pos + scalarLen(c.text);
        if (next > start && pos < end) {
            any = true;
            if (c.style.bold === true) {
                bold = true;
            }
            if (c.style.italic === true) {
                italic = true;
            }
        }
        pos = next;
    }
    return { bold: any && bold, italic: any && italic };
}

function onFormatSelect(
    blockId: string,
    payload: { start: number; end: number; rect: DOMRect },
) {
    const block = blocks.value.find((b) => b.id === blockId);
    if (!block) {
        return;
    }
    const { bold, italic } = selectionStyleInRange(
        block,
        payload.start,
        payload.end,
    );
    formatting.value = {
        blockId,
        start: payload.start,
        end: payload.end,
        rect: payload.rect,
        bold,
        italic,
    };
}

function onFormatClear() {
    formatting.value = null;
}

function onBlockClick(blockId: string) {
    const blockElement = document.querySelector(`[data-block-id="${blockId}"]`);
    const editable = blockElement?.querySelector('[contenteditable="true"]');
    if (editable instanceof HTMLElement) {
        editable.focus();
    }
}

function blockById(id: string) {
    return blocks.value.find((b) => b.id === id);
}

async function applyFormatBold() {
    const fmt = formatting.value;
    if (!fmt) {
        return;
    }
    const block = blockById(fmt.blockId);
    if (!block) {
        return;
    }
    const { bold } = selectionStyleInRange(block, fmt.start, fmt.end);
    if (bold) {
        await api.patchBlock(props.noteId, fmt.blockId, {
            op: "disable_formatting",
            start: fmt.start,
            end: fmt.end,
            bold: true,
        });
    } else {
        await api.patchBlock(props.noteId, fmt.blockId, {
            op: "enable_formatting",
            start: fmt.start,
            end: fmt.end,
            bold: true,
        });
    }
    await loadBlocks();
    formatting.value = null;
}

async function applyFormatItalic() {
    const fmt = formatting.value;
    if (!fmt) {
        return;
    }
    const block = blockById(fmt.blockId);
    if (!block) {
        return;
    }
    const { italic } = selectionStyleInRange(block, fmt.start, fmt.end);
    if (italic) {
        await api.patchBlock(props.noteId, fmt.blockId, {
            op: "disable_formatting",
            start: fmt.start,
            end: fmt.end,
            italic: true,
        });
    } else {
        await api.patchBlock(props.noteId, fmt.blockId, {
            op: "enable_formatting",
            start: fmt.start,
            end: fmt.end,
            italic: true,
        });
    }
    await loadBlocks();
    formatting.value = null;
}
</script>

<style scoped>
.note-editor {
    min-height: 100%;
    color: var(--text-primary);
}

.editor-titlebar {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 24px;
    padding: 48px 32px 20px;
}

.editor-kicker {
    margin: 0 0 8px;
    color: var(--text-disabled);
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 0.12em;
    line-height: 24px;
    text-transform: uppercase;
}

.editor-title {
    margin: 0;
    color: var(--text-primary);
    font-family: var(--font-heading);
    font-size: 28px;
    font-weight: 700;
    letter-spacing: -0.025em;
    line-height: 36px;
}

.add-block {
    min-height: 36px;
    padding: 6px 12px;
    font-size: 13px;
}

.editor-messages {
    display: grid;
    gap: 8px;
    max-width: 760px;
    padding: 0 32px;
}

.block-canvas {
    padding: 0 32px 48px;
}

.block-list {
    display: flex;
    flex-direction: column;
    width: min(760px, 100%);
}

.empty-editor {
    display: grid;
    width: min(760px, 100%);
    min-height: 260px;
    place-content: start;
    padding-top: 8px;
}

.empty-editor p {
    margin: 0 0 16px;
    color: var(--text-disabled);
    font-size: 16px;
    line-height: 24px;
}

.empty-editor .btn {
    justify-self: start;
    min-height: 36px;
    padding: 6px 12px;
    font-size: 13px;
}

.drop-slot {
    position: relative;
    height: 6px;
    width: 100%;
}

.drop-hitbox {
    position: absolute;
    inset: -12px 0;
    z-index: 1;
}

.drop-line {
    position: absolute;
    inset: 2px 8px auto;
    height: 2px;
    border-radius: var(--r-pill);
    background: transparent;
    transition: background-color 150ms ease;
}

.drop-line.is-active {
    background: color-mix(in srgb, var(--accent-primary) 55%, transparent);
}

.block-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 32px;
    gap: 4px;
    align-items: start;
    transition:
        opacity 150ms ease,
        transform 150ms ease;
}

.block-row.is-dragging {
    opacity: 0.6;
    transform: scale(0.995);
}

.block-shell {
    min-width: 0;
    cursor: text;
}

.delete-block {
    margin-top: 4px;
    color: var(--text-disabled);
    opacity: 0;
}

.block-row:hover .delete-block {
    opacity: 1;
}

.delete-block:hover {
    color: var(--danger);
}

@media (max-width: 768px) {
    .editor-titlebar {
        padding: 32px 20px 16px;
    }

    .block-canvas,
    .editor-messages {
        padding-inline: 20px;
    }

    .editor-titlebar {
        flex-direction: column;
    }
}
</style>
