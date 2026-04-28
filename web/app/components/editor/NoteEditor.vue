<template>
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
                Add text block
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
            class="p-[12px] flex justify-center"
            @dragend="onDragEnd"
        >
            <p
                v-if="textBlocks.length === 0"
                class="px-2 py-6 text-center text-[14px] leading-6 text-fg-muted"
            >
                No blocks yet. Add one to start.
            </p>

            <div v-else class="flex flex-col w-full max-w-[512px]">
                <template v-for="(block, index) in textBlocks" :key="block.id">
                    <div
                        class="relative h-[4px] w-full transition-all duration-300 ease-out"
                        @dragover.prevent="onDragOverSlot(index)"
                        @dragleave="onDragLeaveSlot(index)"
                        @drop.prevent="onDrop(index)"
                    >
                        <div class="absolute -inset-y-3 inset-x-0 z-10" />
                        <div
                            class="absolute inset-x-[8px] top-[1px] h-[2px] rounded-full transition-colors"
                            :class="
                                dragActive && dropSlot === index
                                    ? 'bg-blue/40'
                                    : 'bg-transparent'
                            "
                        />
                    </div>

                    <div
                        class="group flex gap-1 transition-all duration-300 ease-out"
                        :class="draggingId === block.id ? 'opacity-60' : ''"
                    >
                        <div
                            class="min-w-0 flex-1 cursor-text"
                            @click.self="onBlockClick(block.id)"
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
                        <button
                            type="button"
                            class="mt-1 h-8 shrink-0 self-start rounded-md px-2 text-[12px] leading-6 text-fg-muted opacity-0 transition-opacity hover:bg-red/20 hover:text-red group-hover:opacity-100"
                            aria-label="Delete block"
                            @click="removeBlock(block.id)"
                        >
                            Delete
                        </button>
                    </div>
                </template>

                <div
                    class="relative h-[4px] w-full transition-all duration-300 ease-out"
                    @dragover.prevent="onDragOverSlot(textBlocks.length)"
                    @dragleave="onDragLeaveSlot(textBlocks.length)"
                    @drop.prevent="onDrop(textBlocks.length)"
                >
                    <div class="absolute -inset-y-3 inset-x-0 z-10" />
                    <div
                        class="absolute inset-x-[8px] top-[1px] h-[2px] rounded-full transition-colors"
                        :class="
                            dragActive && dropSlot === textBlocks.length
                                ? 'bg-blue/40'
                                : 'bg-transparent'
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
    // Focus the EditorTextBlock - it will handle focusing the text
    const blockElement = document.querySelector(`[data-block-id="${blockId}"]`);
    if (blockElement instanceof HTMLElement) {
        blockElement.click();
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
