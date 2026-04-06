<template>
    <div class="flex min-w-0 flex-1 items-start gap-1">
        <button
            type="button"
            class="mt-0.5 flex h-6 w-6 shrink-0 cursor-grab touch-none items-center justify-center rounded text-fg-muted opacity-0 transition-opacity hover:bg-bg-overlay hover:text-fg-secondary active:cursor-grabbing group-hover:opacity-100"
            draggable="true"
            aria-label="Переместить блок"
            @dragstart="onHandleDragStart"
            @click.prevent
        >
            <span class="text-xs leading-none tracking-tighter">⋮⋮</span>
        </button>
        <div
            class="min-w-0 flex-1 rounded-md px-2 py-1 transition-colors"
            :class="isDragging ? 'bg-bg-overlay/50' : 'hover:bg-bg-overlay/40'"
        >
            <div
                ref="editableRef"
                class="editor-text whitespace-pre-wrap break-words text-fg-primary outline-none"
                contenteditable="true"
                spellcheck="false"
                @input="onEditorInput"
                @blur="onBlur"
                @compositionstart="isComposing = true"
                @compositionend="onCompositionEnd"
                @mouseup="reportSelection"
                @keyup="reportSelection"
            />
        </div>
    </div>
</template>

<script setup lang="ts">
import type { NoteBlock, TextChunk, TextStyle } from "~/types/editor";
import { getTextContent, plainFromChunks, scalarLen } from "~/types/editor";

const DEBOUNCE_MS = 300;

const props = defineProps<{
    noteId: string;
    block: NoteBlock;
    isDragging: boolean;
}>();

const emit = defineEmits<{
    "blocks-updated": [blocks: NoteBlock[]];
    "format-select": [payload: { start: number; end: number; rect: DOMRect }];
    "format-clear": [];
    "drag-start": [blockId: string];
}>();

const api = useCoreApi();
const editableRef = ref<HTMLElement | null>(null);
const isComposing = ref(false);
const dirty = ref(false);
const lastSyncedPlain = ref("");
let debounceTimer: ReturnType<typeof setTimeout> | null = null;
let patchTail = Promise.resolve();

function enqueuePatch(fn: () => Promise<void>) {
    patchTail = patchTail.then(fn).catch(console.error);
}

function normalizePlain(s: string): string {
    return s.replace(/\u200b/g, "");
}

/** Plain text from the editor DOM (preserves leading/trailing/repeated spaces). `innerText` collapses them. */
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
        const el = node as HTMLElement;
        if (el.tagName === "BR") {
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

function plainFromBlock(): string {
    return normalizePlain(plainFromChunks(getTextContent(props.block.content)?.chunks ?? []));
}

function escapeHtml(text: string) {
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
    return parts.length ? ` style="${parts.join(";")}"` : "";
}

function syncDomFromChunks() {
    const el = editableRef.value;
    if (!el) {
        return;
    }
    const tc = getTextContent(props.block.content);
    const chunks = tc?.chunks ?? [];
    if (chunks.length === 0 || (chunks.length === 1 && chunks[0].text === "")) {
        el.innerHTML = '<span class="text-fg-muted">&#x200b;</span>';
        return;
    }
    el.innerHTML = chunks
        .map((c) => {
            const cls = [c.style.bold ? "font-semibold" : "", c.style.italic ? "italic" : ""]
                .filter(Boolean)
                .join(" ");
            const cs = chunkStyleAttr(c.style);
            return `<span class="${cls}"${cs}>${escapeHtml(c.text)}</span>`;
        })
        .join("");
}

watch(
    () => props.block,
    () => {
        if (!isComposing.value && !dirty.value) {
            nextTick(() => {
                syncDomFromChunks();
                lastSyncedPlain.value = plainFromBlock();
            });
        }
    },
    { deep: true },
);

onMounted(() => {
    syncDomFromChunks();
    lastSyncedPlain.value = plainFromBlock();
});

/** UTF-16 offset within `s` for a given Unicode-scalar index. */
function utf16OffsetAtScalarIndex(s: string, scalarIdx: number): number {
    let u = 0;
    let si = 0;
    for (const ch of s) {
        if (si >= scalarIdx) {
            return u;
        }
        u += ch.length;
        si++;
    }
    return u;
}

/** Caret position as Unicode-scalar offset from editor start (uses focus for bidirectional selections). */
function getCaretScalarOffset(): number {
    const el = editableRef.value;
    if (!el) {
        return 0;
    }
    const sel = window.getSelection();
    if (!sel || sel.rangeCount === 0 || !sel.focusNode || !el.contains(sel.focusNode)) {
        return scalarLen(normalizePlain(collectTextFromEditor(el)));
    }
    const r = document.createRange();
    r.setStart(el, 0);
    try {
        r.setEnd(sel.focusNode, sel.focusOffset);
    } catch {
        return scalarLen(normalizePlain(collectTextFromEditor(el)));
    }
    return scalarLen(r.toString());
}

function plainOffsetScalars(): { start: number; end: number } {
    const el = editableRef.value;
    if (!el) {
        return { start: 0, end: 0 };
    }
    const sel = window.getSelection();
    if (!sel || sel.rangeCount === 0 || !el.contains(sel.anchorNode)) {
        return { start: 0, end: 0 };
    }
    const range = sel.getRangeAt(0);
    const rStart = document.createRange();
    rStart.setStart(el, 0);
    rStart.setEnd(range.startContainer, range.startOffset);
    const rEnd = document.createRange();
    rEnd.setStart(el, 0);
    rEnd.setEnd(range.endContainer, range.endOffset);
    const start = scalarLen(rStart.toString());
    const end = scalarLen(rEnd.toString());
    return { start, end: Math.max(start, end) };
}

function setCaretPlainOffset(offset: number) {
    const el = editableRef.value;
    if (!el) {
        return;
    }
    const sel = window.getSelection();
    const range = document.createRange();
    let seenScalars = 0;

    function walk(node: Node): boolean {
        if (node.nodeType === Node.TEXT_NODE) {
            const t = node.textContent ?? "";
            const nScalars = scalarLen(t);
            if (seenScalars + nScalars >= offset) {
                const localScalar = offset - seenScalars;
                const u16 = utf16OffsetAtScalarIndex(t, localScalar);
                range.setStart(node, Math.min(u16, t.length));
                range.collapse(true);
                return true;
            }
            seenScalars += nScalars;
        } else {
            for (const ch of Array.from(node.childNodes)) {
                if (walk(ch)) {
                    return true;
                }
            }
        }
        return false;
    }

    if (!walk(el)) {
        range.selectNodeContents(el);
        range.collapse(false);
    }
    sel?.removeAllRanges();
    sel?.addRange(range);
}

function pickStylePayload(style: TextStyle): Record<string, string | boolean> {
    const out: Record<string, string | boolean> = {};
    if (style.bold === true) {
        out.bold = true;
    }
    if (style.italic === true) {
        out.italic = true;
    }
    if (style.color && /^#[0-9A-Fa-f]{3,8}$/.test(style.color)) {
        out.color = style.color;
    }
    return out;
}

function stylePayloadAtScalarOffset(chunks: TextChunk[], offset: number): Record<string, string | boolean> {
    if (chunks.length === 0) {
        return {};
    }
    let pos = 0;
    for (const c of chunks) {
        const next = pos + scalarLen(c.text);
        if (offset < next || (scalarLen(c.text) === 0 && offset === pos)) {
            return pickStylePayload(c.style);
        }
        pos = next;
    }
    return pickStylePayload(chunks[chunks.length - 1].style);
}

function diffPlain(
    oldS: string,
    newS: string,
): { start: number; deleteCount: number; insert: string } | null {
    if (oldS === newS) {
        return null;
    }
    const oldC = [...oldS];
    const newC = [...newS];
    let i = 0;
    const minLen = Math.min(oldC.length, newC.length);
    while (i < minLen && oldC[i] === newC[i]) {
        i++;
    }
    let o = oldC.length - 1;
    let n = newC.length - 1;
    while (o >= i && n >= i && oldC[o] === newC[n]) {
        o--;
        n--;
    }
    const deleteCount = o - i + 1;
    const insert = newC.slice(i, n + 1).join("");
    if (deleteCount === 0 && insert.length === 0) {
        return null;
    }
    return { start: i, deleteCount, insert };
}

function getEditorPlain(): string {
    const el = editableRef.value;
    if (!el) {
        return "";
    }
    return normalizePlain(collectTextFromEditor(el));
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
    const el = editableRef.value;
    if (!el) {
        dirty.value = false;
        return;
    }

    enqueuePatch(async () => {
        const newPlain = getEditorPlain();
        const oldPlain = lastSyncedPlain.value;
        const diff = diffPlain(oldPlain, newPlain);
        if (!diff) {
            dirty.value = false;
            return;
        }

        const chunks = getTextContent(props.block.content)?.chunks ?? [];

        try {
            const { start, deleteCount, insert } = diff;
            if (deleteCount > 0) {
                await api.patchBlock(props.noteId, props.block.id, {
                    op: "delete_range",
                    start,
                    end: start + deleteCount,
                });
            }
            if (insert.length > 0) {
                const stylePayload = stylePayloadAtScalarOffset(chunks, start);
                await api.patchBlock(props.noteId, props.block.id, {
                    op: "insert_text",
                    position: start,
                    text: insert,
                    ...stylePayload,
                });
            }
            const { blocks } = await api.listBlocks(props.noteId);
            emit("blocks-updated", blocks);
            await nextTick();
            const caretScalar = getCaretScalarOffset();
            const self = blocks.find((b) => b.id === props.block.id);
            if (self) {
                lastSyncedPlain.value = normalizePlain(
                    plainFromChunks(getTextContent(self.content)?.chunks ?? []),
                );
            } else {
                lastSyncedPlain.value = getEditorPlain();
            }
            dirty.value = false;
            syncDomFromChunks();
            await nextTick();
            const root = editableRef.value;
            if (root) {
                root.focus();
                const maxLen = scalarLen(lastSyncedPlain.value);
                setCaretPlainOffset(Math.min(caretScalar, maxLen));
            }
        } catch (e) {
            console.error(e);
            dirty.value = false;
            syncDomFromChunks();
            lastSyncedPlain.value = plainFromBlock();
        }
    });
}

function onEditorInput() {
    scheduleSync();
}

function onBlur() {
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
    const el = editableRef.value;
    if (!el) {
        return;
    }
    const sel = window.getSelection();
    if (!sel || sel.rangeCount === 0 || !el.contains(sel.anchorNode)) {
        emit("format-clear");
        return;
    }
    const { start, end } = plainOffsetScalars();
    if (start === end) {
        emit("format-clear");
        return;
    }
    const range = sel.getRangeAt(0);
    emit("format-select", { start, end, rect: range.getBoundingClientRect() });
}

function onHandleDragStart(ev: DragEvent) {
    ev.dataTransfer?.setData("application/x-notalking-block", props.block.id);
    ev.dataTransfer?.setData("text/plain", props.block.id);
    if (ev.dataTransfer) {
        ev.dataTransfer.effectAllowed = "move";
    }
    emit("drag-start", props.block.id);
}
</script>

<style scoped>
.editor-text {
    font-size: 14px;
    line-height: 24px;
}
</style>
