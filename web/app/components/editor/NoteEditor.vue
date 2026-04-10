<template>
    <p>NOTE EDITOR</p>
    <div class="note-editor text-fg-primary">
        <header class="mb-6 flex flex-wrap items-center justify-between gap-3">
            <h2 class="font-rounded text-lg text-fg-primary">
                {{ noteTitle }}
            </h2>
            <button
                type="button"
                class="rounded-md bg-bg-overlay px-3 py-1.5 text-[14px] leading-6 text-fg-secondary hover:bg-bg-float hover:text-fg-primary"
                @click="addTextBlock"
            >
                Добавить текстовый блок
            </button>
        </header>

        <p
            v-if="loadError"
            class="mb-4 rounded-md border border-red/40 bg-red/10 px-3 py-2 text-[14px] leading-6 text-red"
        >
            {{ loadError }}
        </p>
        <p
            v-if="blockActionError"
            class="mb-4 rounded-md border border-red/40 bg-red/10 px-3 py-2 text-[14px] leading-6 text-red"
        >
            {{ blockActionError }}
        </p>

        <div
            class="rounded-lg border border-bg-overlay bg-bg-raised p-2 sm:p-3"
            @dragend="onDragEnd"
        >
            <p
                v-if="textBlocks.length === 0"
                class="px-2 py-6 text-center text-[14px] leading-6 text-fg-muted"
            >
                Нет блоков. Добавьте первый.
            </p>

            <div v-else class="flex flex-col">
                <template v-for="(block, index) in textBlocks" :key="block.id">
                    <div
                        class="relative min-h-2 rounded transition-colors"
                        :class="
                            dragActive && dropSlot === index
                                ? 'bg-blue/25 ring-1 ring-blue/40'
                                : 'bg-transparent'
                        "
                        @dragover.prevent="onDragOverSlot(index)"
                        @dragleave="onDragLeaveSlot(index)"
                        @drop.prevent="onDrop(index)"
                    >
                        <div
                            class="absolute inset-x-2 top-1/2 h-0.5 -translate-y-1/2 rounded bg-blue/60 opacity-0 transition-opacity"
                            :class="
                                dragActive && dropSlot === index
                                    ? 'opacity-100'
                                    : ''
                            "
                        />
                    </div>

                    <div
                        class="group rounded-md p-1.5 transition-colors sm:p-2"
                        :class="draggingId === block.id ? 'opacity-60' : ''"
                    >
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
                </template>

                <div
                    class="relative min-h-2 rounded transition-colors"
                    :class="
                        dragActive && dropSlot === textBlocks.length
                            ? 'bg-blue/25 ring-1 ring-blue/40'
                            : 'bg-transparent'
                    "
                    @dragover.prevent="onDragOverSlot(textBlocks.length)"
                    @dragleave="onDragLeaveSlot(textBlocks.length)"
                    @drop.prevent="onDrop(textBlocks.length)"
                >
                    <div
                        class="absolute inset-x-2 top-1/2 h-0.5 -translate-y-1/2 rounded bg-blue/60 opacity-0 transition-opacity"
                        :class="
                            dragActive && dropSlot === textBlocks.length
                                ? 'opacity-100'
                                : ''
                        "
                    />
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
    </div>
</template>

<script setup lang="ts">
import type { NoteBlock } from "~/types/editor";
import { getTextContent, scalarLen } from "~/types/editor";

const props = defineProps<{
    noteId: string;
    noteTitle: string;
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

function formatFetchError(e: unknown): string {
    const err = e as {
        data?: { error?: string; message?: string };
        message?: string;
        statusCode?: number;
        cause?: { message?: string };
    };
    const msg =
        err?.data?.error ??
        err?.data?.message ??
        err?.message ??
        err?.cause?.message;
    if (msg) {
        if (/fetch failed|ECONNREFUSED|connect/i.test(msg)) {
            return "Не удаётся связаться с Core API (порт 40000). Запустите core и проверьте прокси /core.";
        }
        return msg;
    }
    if (err?.statusCode === 401) {
        return "Сессия истекла или не авторизованы. Войдите снова.";
    }
    if (err?.statusCode === 502 || err?.statusCode === 503) {
        return "Core API недоступен. Запустите сервис (порт 40000) и проверьте прокси /core.";
    }
    return "Не удалось загрузить блоки заметки.";
}

async function loadBlocks() {
    if (import.meta.server) {
        return;
    }
    loadError.value = "";
    try {
        const res = await api.listBlocks(props.noteId);
        const list = Array.isArray(res?.blocks) ? res.blocks : [];
        blocks.value = list as NoteBlock[];
    } catch (e) {
        blocks.value = [];
        loadError.value = formatFetchError(e);
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
    } catch (e) {
        blockActionError.value = formatFetchError(e);
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
            const afterId = list[slotIndex - 1].id;
            if (afterId === dragged) {
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
