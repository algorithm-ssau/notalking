<template>
    <article class="note-editor">
        <header class="editor-titlebar">
            <div class="editor-title-wrap">
                <input
                    v-model="titleDraft"
                    class="editor-title-input"
                    type="text"
                    spellcheck="false"
                    placeholder="Untitled note"
                    @focus="isEditingTitle = true"
                    @blur="commitTitle"
                    @keydown.enter.prevent="commitTitle"
                    @keydown.esc.prevent="resetTitleDraft"
                >
                <p v-if="titleError" class="title-error">{{ titleError }}</p>
            </div>
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
                        :class="['drop-slot', { 'is-active': dragActive && dropSlot === index }]"
                        @dragover.prevent="onDragOverSlot(index)"
                        @dragleave="onDragLeaveSlot(index)"
                        @drop.prevent="onDrop(index)"
                    >
                        <div class="drop-hitbox" />
                        <div class="drop-line" />
                    </div>

                    <div :class="['block-row', { 'is-dragging': draggingId === block.id }]">
                        <div class="block-shell" @click.self="onBlockClick(block.id)">
                            <EditorTextBlock
                                :note-id="noteId"
                                :block="block"
                                :is-dragging="draggingId === block.id"
                                :selection-restore="selectionRestore?.blockId === block.id ? selectionRestore : null"
                                @block-updated="onBlockUpdated"
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
                    :class="['drop-slot', { 'is-active': dragActive && dropSlot === textBlocks.length }]"
                    @dragover.prevent="onDragOverSlot(textBlocks.length)"
                    @dragleave="onDragLeaveSlot(textBlocks.length)"
                    @drop.prevent="onDrop(textBlocks.length)"
                >
                    <div class="drop-hitbox" />
                    <div class="drop-line" />
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
import type { NoteResponse } from "~/types/core";
import type { NoteBlock } from "~/types/editor";
import {
    getTextContent,
    replaceBlockChunks,
    scalarLen,
    setFormattingOnChunks,
} from "~/types/editor";
import { getCoreErrorMessage } from "~/utils/coreErrors";

const props = withDefaults(
    defineProps<{
        noteId: string;
        noteTitle: string;
        focusBlockId?: string | null;
        reloadNonce?: number;
    }>(),
    {
        focusBlockId: null,
        reloadNonce: 0,
    },
);

const emit = defineEmits<{
    "focus-block-done": [];
    "note-updated": [note: NoteResponse];
}>();

const api = useCoreApi();
const blocks = ref<NoteBlock[]>([]);
const loadError = ref("");
const blockActionError = ref("");
const titleError = ref("");
const titleDraft = ref("");
const isEditingTitle = ref(false);
const draggingId = ref<string | null>(null);
const dropSlot = ref<number | null>(null);
const selectionToken = ref(0);
const selectionRestore = ref<{
    blockId: string;
    token: number;
    anchor: number;
    focus: number;
} | null>(null);
const formatting = ref<{
    blockId: string;
    anchor: number;
    focus: number;
    start: number;
    end: number;
    rect: DOMRect;
    bold: boolean;
    italic: boolean;
} | null>(null);

const dragActive = computed(() => draggingId.value != null);
const textBlocks = computed(() => blocks.value.filter((block) => getTextContent(block.content) !== null));

watch(
    () => props.noteId,
    () => {
        resetTitleDraft();
        void loadBlocks();
    },
    { immediate: true },
);

watch(
    () => props.reloadNonce,
    () => {
        void loadBlocks();
    },
);

watch(
    () => props.noteTitle,
    () => {
        if (!isEditingTitle.value) {
            resetTitleDraft();
        }
    },
);

watch(
    [() => props.focusBlockId, blocks],
    async () => {
        const id = props.focusBlockId;
        if (!id || import.meta.server || !blocks.value.some((block) => block.id === id)) {
            return;
        }
        await nextTick();
        const element = document.querySelector(`[data-block-id="${id}"]`);
        element?.scrollIntoView({ behavior: "smooth", block: "center" });
        emit("focus-block-done");
    },
    { flush: "post" },
);

function resolvedTitle(value = props.noteTitle): string {
    return value.trim() || "Untitled note";
}

function resetTitleDraft() {
    titleDraft.value = resolvedTitle();
    titleError.value = "";
    isEditingTitle.value = false;
}

async function commitTitle() {
    isEditingTitle.value = false;
    titleError.value = "";
    const nextTitle = resolvedTitle(titleDraft.value);
    titleDraft.value = nextTitle;
    if (nextTitle === resolvedTitle()) {
        return;
    }
    try {
        const note = await api.updateNote(props.noteId, { title: nextTitle });
        emit("note-updated", note);
        titleDraft.value = resolvedTitle(note.title);
    } catch (error: unknown) {
        titleError.value = getCoreErrorMessage(error, "Could not rename note.");
        titleDraft.value = resolvedTitle();
    }
}

async function loadBlocks() {
    if (import.meta.server || !props.noteId) {
        return;
    }
    loadError.value = "";
    try {
        const response = await api.listBlocks(props.noteId);
        blocks.value = Array.isArray(response?.blocks) ? response.blocks as NoteBlock[] : [];
    } catch (error: unknown) {
        blocks.value = [];
        loadError.value = getCoreErrorMessage(error, "Could not load note blocks.");
    }
}

function onBlockUpdated(updated: NoteBlock) {
    blocks.value = blocks.value.map((block) => block.id === updated.id ? updated : block);
}

async function addTextBlock() {
    blockActionError.value = "";
    const afterId = textBlocks.value[textBlocks.value.length - 1]?.id ?? null;
    try {
        const block = await api.createTextBlock(props.noteId, afterId, "");
        blocks.value = [...blocks.value, block];
    } catch (error: unknown) {
        blockActionError.value = getCoreErrorMessage(error, "Could not add block.");
    }
}

async function removeBlock(blockId: string) {
    if (!confirm("Delete this block?")) {
        return;
    }
    blockActionError.value = "";
    try {
        await api.deleteBlock(props.noteId, blockId);
        blocks.value = blocks.value.filter((block) => block.id !== blockId);
        formatting.value = formatting.value?.blockId === blockId ? null : formatting.value;
    } catch (error: unknown) {
        blockActionError.value = getCoreErrorMessage(error, "Could not delete block.");
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

function moveBlockLocally(list: NoteBlock[], fromIndex: number, toIndex: number): NoteBlock[] {
    const next = [...list];
    const [item] = next.splice(fromIndex, 1);
    next.splice(toIndex, 0, item);
    return next;
}

async function onDrop(slotIndex: number) {
    const dragged = draggingId.value;
    if (!dragged) {
        return;
    }
    const list = [...textBlocks.value];
    const fromIndex = list.findIndex((block) => block.id === dragged);
    if (fromIndex < 0) {
        onDragEnd();
        return;
    }

    const targetIndex = slotIndex > fromIndex ? slotIndex - 1 : slotIndex;
    if (targetIndex === fromIndex) {
        onDragEnd();
        return;
    }

    const reordered = moveBlockLocally(list, fromIndex, Math.max(0, Math.min(targetIndex, list.length - 1)));
    const movedIndex = reordered.findIndex((block) => block.id === dragged);
    const previousBlock = movedIndex > 0 ? reordered[movedIndex - 1] : null;
    const nextBlock = reordered[movedIndex + 1] ?? null;

    blockActionError.value = "";
    try {
        if (!previousBlock && nextBlock) {
            await api.patchBlock(props.noteId, dragged, {
                op: "move",
                before_id: nextBlock.id,
            });
        } else {
            await api.patchBlock(props.noteId, dragged, {
                op: "move",
                after_id: previousBlock?.id ?? null,
            });
        }
        blocks.value = reordered;
    } catch (error: unknown) {
        blockActionError.value = getCoreErrorMessage(error, "Could not move block.");
    } finally {
        onDragEnd();
    }
}

function selectionStyleInRange(block: NoteBlock, start: number, end: number) {
    const text = getTextContent(block.content);
    if (!text) {
        return { bold: false, italic: false };
    }
    let pos = 0;
    let bold = false;
    let italic = false;
    let any = false;
    for (const chunk of text.chunks) {
        const next = pos + scalarLen(chunk.text);
        if (next > start && pos < end) {
            any = true;
            if (chunk.style.bold === true) {
                bold = true;
            }
            if (chunk.style.italic === true) {
                italic = true;
            }
        }
        pos = next;
    }
    return { bold: any && bold, italic: any && italic };
}

function onFormatSelect(
    blockId: string,
    payload: { anchor: number; focus: number; start: number; end: number; rect: DOMRect },
) {
    const block = blocks.value.find((item) => item.id === blockId);
    if (!block) {
        return;
    }
    const { bold, italic } = selectionStyleInRange(block, payload.start, payload.end);
    formatting.value = {
        blockId,
        anchor: payload.anchor,
        focus: payload.focus,
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
    return blocks.value.find((block) => block.id === id);
}

async function applyFormatting(style: { bold?: true; italic?: true }, enabled: boolean) {
    const current = formatting.value;
    if (!current) {
        return;
    }
    const block = blockById(current.blockId);
    const text = block ? getTextContent(block.content) : null;
    if (!block || !text) {
        return;
    }

    blockActionError.value = "";
    const nextChunks = setFormattingOnChunks(text.chunks, current.start, current.end, style, enabled);
    const nextBlock = replaceBlockChunks(block, nextChunks);
    onBlockUpdated(nextBlock);

    selectionToken.value += 1;
    selectionRestore.value = {
        blockId: current.blockId,
        token: selectionToken.value,
        anchor: current.anchor,
        focus: current.focus,
    };

    try {
        await api.patchBlock(props.noteId, current.blockId, {
            op: enabled ? "enable_formatting" : "disable_formatting",
            start: current.start,
            end: current.end,
            ...style,
        });
    } catch (error: unknown) {
        blockActionError.value = getCoreErrorMessage(error, "Could not update formatting.");
        await loadBlocks();
    } finally {
        formatting.value = null;
    }
}

async function applyFormatBold() {
    const current = formatting.value;
    if (!current) {
        return;
    }
    await applyFormatting({ bold: true }, !current.bold);
}

async function applyFormatItalic() {
    const current = formatting.value;
    if (!current) {
        return;
    }
    await applyFormatting({ italic: true }, !current.italic);
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

.editor-title-wrap {
    min-width: 0;
    flex: 1;
}

.editor-title-input {
    width: 100%;
    border: 0;
    background: transparent;
    color: var(--text-primary);
    font-family: var(--font-heading);
    font-size: clamp(26px, 4vw, 34px);
    font-weight: 500;
    letter-spacing: -0.04em;
    line-height: 40px;
    outline: none;
    padding: 0;
}

.editor-title-input::placeholder {
    color: var(--text-disabled);
}

.title-error {
    margin: 6px 0 0;
    color: var(--danger);
    font-size: 12px;
    line-height: 18px;
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

.drop-slot {
    position: relative;
    height: 12px;
    width: 100%;
}

.drop-hitbox {
    position: absolute;
    inset: 0;
    z-index: 1;
}

.drop-line {
    position: absolute;
    inset: 5px 0 auto;
    height: 2px;
    border-radius: var(--r-pill);
    background: transparent;
    transition:
        background-color 120ms ease,
        box-shadow 120ms ease,
        transform 120ms ease;
}

.drop-slot.is-active .drop-line {
    background: var(--accent-primary);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent-primary) 18%, transparent);
    transform: scaleY(1.3);
}

.block-row {
    position: relative;
    display: grid;
    grid-template-columns: minmax(0, 1fr) 32px;
    gap: 8px;
    align-items: start;
}

.block-row.is-dragging {
    opacity: 0.58;
}

.delete-block {
    opacity: 0;
    transition: opacity 120ms ease;
}

.block-row:hover .delete-block {
    opacity: 1;
}
</style>
