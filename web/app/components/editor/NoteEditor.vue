<template>
    <article class="note-editor">
        <header class="editor-titlebar">
            <h1>{{ noteTitle || "Untitled note" }}</h1>
            <button class="add-block" type="button" @click="addTextBlock">
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
    padding: 0 0 40px;
}

.editor-titlebar {
    display: flex;
    width: min(584px, calc(100% - 48px));
    align-items: flex-start;
    justify-content: space-between;
    gap: 18px;
    margin: 0 auto;
    padding: 28px 0 12px;
}

.editor-titlebar h1 {
    margin: 0;
    color: var(--text-primary);
    font-family: var(--font-heading);
    font-size: clamp(26px, 4vw, 34px);
    font-weight: 500;
    letter-spacing: -0.04em;
    line-height: 40px;
}

.add-block {
    display: inline-flex;
    min-height: 32px;
    align-items: center;
    gap: 6px;
    border: 1px solid color-mix(in srgb, var(--bg-4) 70%, transparent);
    border-radius: 8px;
    background: transparent;
    color: var(--text-tertiary);
    flex: 0 0 auto;
    margin-top: 2px;
    padding: 4px 12px;
    font-size: 13px;
    font-weight: 500;
    line-height: 20px;
    cursor: pointer;
    transition:
        background-color 150ms ease,
        border-color 150ms ease,
        color 150ms ease;
}

.add-block:hover {
    background: color-mix(in srgb, var(--bg-3) 70%, transparent);
    border-color: color-mix(in srgb, var(--accent-primary) 20%, var(--bg-4));
    color: var(--text-secondary);
}

.editor-messages {
    display: grid;
    width: min(584px, calc(100% - 48px));
    gap: 8px;
    margin: 0 auto 10px;
}

.block-canvas {
    width: min(584px, calc(100% - 48px));
    margin: 0 auto;
    padding: 0;
}

.block-list {
    display: flex;
    flex-direction: column;
    width: 100%;
}

.empty-editor {
    display: grid;
    min-height: 260px;
    place-content: center;
    justify-items: center;
    border: 1px solid color-mix(in srgb, var(--bg-4) 60%, transparent);
    border-radius: 12px;
    background: var(--bg-1);
    padding: 32px;
    text-align: center;
}

.empty-editor p {
    margin: 0 0 16px;
    color: var(--text-tertiary);
    font-size: 16px;
    line-height: 24px;
}

.empty-editor .btn {
    min-height: 36px;
    border-color: transparent;
    background: var(--bg-3);
    color: var(--text-secondary);
    padding: 6px 14px;
    font-size: 13px;
    line-height: 20px;
}

.drop-slot {
    position: relative;
    height: 4px;
    width: 100%;
}

.drop-hitbox {
    position: absolute;
    inset: -10px 0;
    z-index: 1;
}

.drop-line {
    position: absolute;
    inset: 1px 0 auto;
    height: 2px;
    border-radius: var(--r-pill);
    background: transparent;
    transition: background-color 120ms ease;
}

.drop-line.is-active {
    background: var(--accent-primary);
}

.block-row {
    position: relative;
    display: grid;
    grid-template-columns: minmax(0, 1fr) 32px;
    gap: 8px;
    align-items: start;
    transition:
        opacity 120ms ease,
        transform 120ms ease;
}

.block-row.is-dragging {
    opacity: 0.45;
    transform: scale(0.997);
}

.block-shell {
    min-width: 0;
}

.delete-block {
    margin-top: 1px;
    opacity: 0;
}

.block-row:hover .delete-block {
    opacity: 1;
}

.delete-block:hover {
    color: var(--danger);
}

@media (max-width: 760px) {
    .editor-titlebar,
    .editor-messages,
    .block-canvas {
        width: min(584px, calc(100% - 28px));
    }

    .editor-titlebar {
        flex-direction: column;
        padding-top: 22px;
    }

    .block-row {
        grid-template-columns: minmax(0, 1fr) 28px;
    }
}
</style>
