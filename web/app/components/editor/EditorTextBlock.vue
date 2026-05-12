<template>
    <div
        :class="[
            'editor-block',
            {
                'is-dragging': isDragging,
                'is-active': isFocused,
                'is-empty': plainFromBlock().length === 0,
            },
        ]"
        :data-block-id="block.id"
    >
        <button
            type="button"
            class="drag-handle"
            draggable="true"
            aria-label="Move block"
            @dragstart="onHandleDragStart"
            @click.prevent
        >
            <span>⋮⋮</span>
        </button>
        <div
            ref="editableRef"
            class="editor-text"
            contenteditable="true"
            spellcheck="false"
            @click="onEditorClick"
            @focus="isFocused = true"
            @input="onEditorInput"
            @blur="onBlur"
            @compositionstart="isComposing = true"
            @compositionend="onCompositionEnd"
            @mouseup="reportSelection"
            @keyup="reportSelection"
        />
    </div>
</template>

<script setup lang="ts">
import type { NoteBlock, TextChunk, TextStyle } from "~/types/editor";
import {
    cloneChunks,
    deleteRangeFromChunks,
    getTextContent,
    insertTextIntoChunks,
    plainFromChunks,
    replaceBlockChunks,
    scalarLen,
} from "~/types/editor";

const DEBOUNCE_MS = 220;

const props = defineProps<{
    noteId: string;
    block: NoteBlock;
    isDragging: boolean;
    selectionRestore?: {
        token: number;
        anchor: number;
        focus: number;
    } | null;
}>();

const emit = defineEmits<{
    "block-updated": [block: NoteBlock];
    "format-select": [payload: { anchor: number; focus: number; start: number; end: number; rect: DOMRect }];
    "format-clear": [];
    "drag-start": [blockId: string];
}>();

const api = useCoreApi();
const editableRef = ref<HTMLElement | null>(null);
const isComposing = ref(false);
const isFocused = ref(false);
const dirty = ref(false);
const lastSyncedPlain = ref("");
const lastSyncedChunks = ref<TextChunk[]>([]);
const renderedSignature = ref("");
let debounceTimer: ReturnType<typeof setTimeout> | null = null;
let patchTail = Promise.resolve();

function enqueuePatch(fn: () => Promise<void>) {
    patchTail = patchTail.then(fn).catch(console.error);
}

function normalizePlain(value: string): string {
    return value.replace(/\u200b/g, "");
}

function collectTextFromEditor(root: HTMLElement): string {
    let out = "";
    function walk(node: Node) {
        if (node.nodeType === Node.TEXT_NODE) {
            out += node.textContent ?? "";
            return;
        }
        if (node.nodeType !== Node.ELEMENT_NODE) {
            return;
        }
        const element = node as HTMLElement;
        if (element.tagName === "BR") {
            out += "\n";
            return;
        }
        for (const child of Array.from(node.childNodes)) {
            walk(child);
        }
    }
    walk(root);
    return out;
}

function blockChunks(): TextChunk[] {
    return cloneChunks(getTextContent(props.block.content)?.chunks ?? []);
}

function blockSignature(): string {
    return JSON.stringify(blockChunks());
}

function plainFromBlock(): string {
    return normalizePlain(plainFromChunks(blockChunks()));
}

function escapeHtml(text: string): string {
    return text
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;")
        .replace(/"/g, "&quot;");
}

function chunkStyleAttr(style: TextStyle): string {
    const parts: string[] = [];
    if (style.bold) {
        parts.push("font-weight:600");
    }
    if (style.italic) {
        parts.push("font-style:italic");
    }
    if (style.color && /^#[0-9A-Fa-f]{3,8}$/.test(style.color)) {
        parts.push(`color:${style.color}`);
    }
    return parts.length ? ` style="${parts.join(';')}"` : "";
}

function syncDomFromChunks(force = false) {
    const element = editableRef.value;
    if (!element) {
        return;
    }
    const signature = blockSignature();
    if (!force && renderedSignature.value === signature) {
        return;
    }
    const chunks = blockChunks();
    if (chunks.length === 0 || (chunks.length === 1 && chunks[0].text === "")) {
        element.innerHTML = "";
    } else {
        element.innerHTML = chunks.map((chunk) => `<span${chunkStyleAttr(chunk.style)}>${escapeHtml(chunk.text)}</span>`).join("");
    }
    renderedSignature.value = signature;
}

watch(
    () => props.block,
    () => {
        const nextSignature = blockSignature();
        lastSyncedChunks.value = blockChunks();
        lastSyncedPlain.value = plainFromBlock();
        if (!dirty.value && !isComposing.value) {
            nextTick(() => syncDomFromChunks(nextSignature !== renderedSignature.value));
        }
    },
    { deep: true, immediate: true },
);

watch(
    () => props.selectionRestore?.token,
    async (token) => {
        if (!token || !props.selectionRestore) {
            return;
        }
        await nextTick();
        restoreSelection(props.selectionRestore.anchor, props.selectionRestore.focus);
    },
);

onMounted(() => {
    syncDomFromChunks(true);
    lastSyncedChunks.value = blockChunks();
    lastSyncedPlain.value = plainFromBlock();
});

function utf16OffsetAtScalarIndex(text: string, scalarIndex: number): number {
    let utf16 = 0;
    let seen = 0;
    for (const char of text) {
        if (seen >= scalarIndex) {
            return utf16;
        }
        utf16 += char.length;
        seen += 1;
    }
    return utf16;
}

function resolvePointAtScalarOffset(root: HTMLElement, offset: number): { node: Node; offset: number } {
    let seen = 0;
    const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
    let current = walker.nextNode();
    while (current) {
        const text = current.textContent ?? "";
        const scalarCount = scalarLen(text);
        if (seen + scalarCount >= offset) {
            const local = offset - seen;
            return {
                node: current,
                offset: Math.min(utf16OffsetAtScalarIndex(text, local), text.length),
            };
        }
        seen += scalarCount;
        current = walker.nextNode();
    }
    return { node: root, offset: root.childNodes.length };
}

function rangeOffset(root: HTMLElement, node: Node, offset: number): number {
    const range = document.createRange();
    range.setStart(root, 0);
    try {
        range.setEnd(node, offset);
    } catch {
        return scalarLen(normalizePlain(collectTextFromEditor(root)));
    }
    return scalarLen(range.toString());
}

function selectionSnapshot(): { anchor: number; focus: number; start: number; end: number } | null {
    const element = editableRef.value;
    const selection = window.getSelection();
    if (!element || !selection || selection.rangeCount === 0 || !selection.anchorNode || !element.contains(selection.anchorNode)) {
        return null;
    }
    const anchor = rangeOffset(element, selection.anchorNode, selection.anchorOffset);
    const focus = selection.focusNode && element.contains(selection.focusNode)
        ? rangeOffset(element, selection.focusNode, selection.focusOffset)
        : anchor;
    return {
        anchor,
        focus,
        start: Math.min(anchor, focus),
        end: Math.max(anchor, focus),
    };
}

function restoreSelection(anchor: number, focus: number) {
    const element = editableRef.value;
    const selection = window.getSelection();
    if (!element || !selection) {
        return;
    }
    const anchorPoint = resolvePointAtScalarOffset(element, anchor);
    const focusPoint = resolvePointAtScalarOffset(element, focus);
    selection.removeAllRanges();
    if (typeof selection.setBaseAndExtent === "function") {
        selection.setBaseAndExtent(anchorPoint.node, anchorPoint.offset, focusPoint.node, focusPoint.offset);
        return;
    }
    const range = document.createRange();
    range.setStart(anchorPoint.node, anchorPoint.offset);
    range.setEnd(focusPoint.node, focusPoint.offset);
    selection.addRange(range);
}

function pickStylePayload(style: TextStyle): Record<string, string | boolean> {
    const output: Record<string, string | boolean> = {};
    if (style.bold === true) {
        output.bold = true;
    }
    if (style.italic === true) {
        output.italic = true;
    }
    if (style.color && /^#[0-9A-Fa-f]{3,8}$/.test(style.color)) {
        output.color = style.color;
    }
    return output;
}

function stylePayloadAtScalarOffset(chunks: TextChunk[], offset: number): Record<string, string | boolean> {
    if (chunks.length === 0) {
        return {};
    }
    let position = 0;
    for (const chunk of chunks) {
        const next = position + scalarLen(chunk.text);
        if (offset < next || (scalarLen(chunk.text) === 0 && offset === position)) {
            return pickStylePayload(chunk.style);
        }
        position = next;
    }
    return pickStylePayload(chunks[chunks.length - 1]?.style ?? {});
}

function diffPlain(oldValue: string, newValue: string): { start: number; deleteCount: number; insert: string } | null {
    if (oldValue === newValue) {
        return null;
    }
    const oldChars = [...oldValue];
    const newChars = [...newValue];
    let start = 0;
    const minLen = Math.min(oldChars.length, newChars.length);
    while (start < minLen && oldChars[start] === newChars[start]) {
        start += 1;
    }
    let oldIndex = oldChars.length - 1;
    let newIndex = newChars.length - 1;
    while (oldIndex >= start && newIndex >= start && oldChars[oldIndex] === newChars[newIndex]) {
        oldIndex -= 1;
        newIndex -= 1;
    }
    return {
        start,
        deleteCount: oldIndex - start + 1,
        insert: newChars.slice(start, newIndex + 1).join(""),
    };
}

function getEditorPlain(): string {
    const element = editableRef.value;
    if (!element) {
        return "";
    }
    return normalizePlain(collectTextFromEditor(element));
}

function scheduleSync() {
    if (isComposing.value) {
        return;
    }
    dirty.value = true;
    if (debounceTimer != null) {
        clearTimeout(debounceTimer);
    }
    debounceTimer = setTimeout(() => {
        debounceTimer = null;
        void flushDebouncedSync();
    }, DEBOUNCE_MS);
}

async function flushDebouncedSync() {
    const element = editableRef.value;
    if (!element) {
        dirty.value = false;
        return;
    }

    const snapshot = selectionSnapshot();
    enqueuePatch(async () => {
        const newPlain = getEditorPlain();
        const oldPlain = lastSyncedPlain.value;
        const diff = diffPlain(oldPlain, newPlain);
        if (!diff) {
            dirty.value = false;
            return;
        }

        const baseChunks = cloneChunks(lastSyncedChunks.value);
        try {
            let nextChunks = baseChunks;
            if (diff.deleteCount > 0) {
                await api.patchBlock(props.noteId, props.block.id, {
                    op: "delete_range",
                    start: diff.start,
                    end: diff.start + diff.deleteCount,
                });
                nextChunks = deleteRangeFromChunks(nextChunks, diff.start, diff.start + diff.deleteCount);
            }
            if (diff.insert.length > 0) {
                const stylePayload = stylePayloadAtScalarOffset(baseChunks, diff.start);
                await api.patchBlock(props.noteId, props.block.id, {
                    op: "insert_text",
                    position: diff.start,
                    text: diff.insert,
                    ...stylePayload,
                });
                nextChunks = insertTextIntoChunks(nextChunks, diff.start, diff.insert, stylePayload as TextStyle);
            }
            lastSyncedChunks.value = cloneChunks(nextChunks);
            lastSyncedPlain.value = plainFromChunks(nextChunks);
            renderedSignature.value = JSON.stringify(nextChunks);
            emit("block-updated", replaceBlockChunks(props.block, nextChunks));
            dirty.value = false;
            if (snapshot && isFocused.value) {
                await nextTick();
                restoreSelection(snapshot.anchor, snapshot.focus);
            }
        } catch (error) {
            console.error(error);
            dirty.value = false;
            syncDomFromChunks(true);
            lastSyncedChunks.value = blockChunks();
            lastSyncedPlain.value = plainFromBlock();
        }
    });
}

function onEditorInput() {
    scheduleSync();
}

function onBlur() {
    isFocused.value = false;
    emit("format-clear");
    if (debounceTimer != null) {
        clearTimeout(debounceTimer);
        debounceTimer = null;
    }
    void flushDebouncedSync();
}

function onCompositionEnd() {
    isComposing.value = false;
    scheduleSync();
}

function reportSelection() {
    const element = editableRef.value;
    const selection = window.getSelection();
    if (!element || !selection || selection.rangeCount === 0 || !selection.anchorNode || !element.contains(selection.anchorNode)) {
        emit("format-clear");
        return;
    }
    const snapshot = selectionSnapshot();
    if (!snapshot || snapshot.start === snapshot.end) {
        emit("format-clear");
        return;
    }
    const range = selection.getRangeAt(0);
    emit("format-select", {
        anchor: snapshot.anchor,
        focus: snapshot.focus,
        start: snapshot.start,
        end: snapshot.end,
        rect: range.getBoundingClientRect(),
    });
}

function onHandleDragStart(event: DragEvent) {
    event.dataTransfer?.setData("application/x-notalking-block", props.block.id);
    event.dataTransfer?.setData("text/plain", props.block.id);
    if (event.dataTransfer) {
        event.dataTransfer.effectAllowed = "move";
    }
    emit("drag-start", props.block.id);
}

function onEditorClick(event: MouseEvent) {
    event.stopPropagation();
}
</script>

<style scoped>
.editor-block {
    position: relative;
    min-width: 0;
    flex: 1;
    border-radius: 7px;
    padding: 2px 4px 2px 0;
    transition:
        background-color 120ms ease,
        opacity 120ms ease;
}

.editor-block:hover,
.editor-block.is-dragging,
.editor-block.is-active {
    background: rgb(255 255 255 / 0.025);
}

.drag-handle {
    position: absolute;
    top: 4px;
    left: -30px;
    display: flex;
    width: 22px;
    height: 22px;
    align-items: center;
    justify-content: center;
    border: 0;
    border-radius: 5px;
    background: transparent;
    color: var(--text-disabled);
    cursor: grab;
    opacity: 0;
    transition:
        background-color 120ms ease,
        color 120ms ease,
        opacity 120ms ease;
}

.drag-handle:hover {
    background: var(--bg-3);
    color: var(--text-secondary);
}

.drag-handle:active {
    cursor: grabbing;
}

.editor-block:hover .drag-handle,
.editor-block.is-active .drag-handle {
    opacity: 1;
}

.drag-handle span {
    font-size: 12px;
    letter-spacing: -0.12em;
    line-height: 1;
}

.editor-text {
    min-height: 24px;
    color: var(--text-primary);
    font-family: var(--font-body);
    font-size: 16px;
    line-height: 24px;
    outline: none;
    overflow-wrap: anywhere;
    white-space: pre-wrap;
}

.editor-block.is-empty .editor-text::before {
    content: 'Start writing...';
    color: var(--text-disabled);
    pointer-events: none;
}

@media (max-width: 760px) {
    .drag-handle {
        left: -6px;
    }
}
</style>
