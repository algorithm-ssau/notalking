export interface TextStyle {
    bold?: boolean | null;
    italic?: boolean | null;
    color?: string | null;
}

export interface TextChunk {
    text: string;
    style: TextStyle;
}

export interface TextContent {
    chunks: TextChunk[];
}

export type BlockContent =
    | { text: TextContent }
    | { Text: TextContent };

export interface NoteBlock {
    id: string;
    prev_id: string | null;
    next_id: string | null;
    content: BlockContent;
    metadata?: unknown;
    created_at: string;
    updated_at: string;
}

export interface NoteSummary {
    id: string;
    title: string;
    created_at: string;
    updated_at: string;
}

export interface BlockSelectionSnapshot {
    anchor: number;
    focus: number;
    start: number;
    end: number;
}

function normalizeTextPayload(inner: object): TextContent {
    const raw = inner as { chunks?: unknown };
    const chunks = raw.chunks;
    return {
        chunks: Array.isArray(chunks) ? (chunks as TextChunk[]) : [],
    };
}

/** Returns structured text for a `Content::Text` block, or null for other / unknown shapes. */
export function getTextContent(content: unknown): TextContent | null {
    if (content == null || typeof content !== "object" || Array.isArray(content)) {
        return null;
    }
    const c = content as Record<string, unknown>;
    const inner = c.text ?? c.Text;
    if (inner == null || typeof inner !== "object" || Array.isArray(inner)) {
        return null;
    }
    return normalizeTextPayload(inner);
}

export function plainFromChunks(chunks: TextChunk[]): string {
    return chunks.map((c) => c.text).join("");
}

/** Unicode scalar count (matches Rust `TextBlock` positions and JS `[...str].length`). */
export function scalarLen(s: string): number {
    return [...s].length;
}

function normalizeStyle(style: TextStyle | null | undefined): TextStyle {
    return {
        ...(style?.bold != null ? { bold: style.bold } : {}),
        ...(style?.italic != null ? { italic: style.italic } : {}),
        ...(style?.color ? { color: style.color } : {}),
    };
}

function sameStyle(a: TextStyle | null | undefined, b: TextStyle | null | undefined): boolean {
    const left = normalizeStyle(a);
    const right = normalizeStyle(b);
    return left.bold === right.bold
        && left.italic === right.italic
        && (left.color || null) === (right.color || null);
}

function cloneChunk(chunk: TextChunk): TextChunk {
    return {
        text: chunk.text,
        style: normalizeStyle(chunk.style),
    };
}

function sliceByScalar(text: string, start: number, end: number): string {
    return [...text].slice(start, end).join("");
}

export function cloneChunks(chunks: TextChunk[]): TextChunk[] {
    return chunks.map(cloneChunk);
}

export function normalizeChunks(chunks: TextChunk[]): TextChunk[] {
    const merged: TextChunk[] = [];
    for (const raw of chunks) {
        if (raw.text.length === 0 && merged.length > 0) {
            continue;
        }
        const chunk = cloneChunk(raw);
        const prev = merged[merged.length - 1];
        if (prev && sameStyle(prev.style, chunk.style)) {
            prev.text += chunk.text;
            continue;
        }
        if (chunk.text.length > 0 || merged.length === 0) {
            merged.push(chunk);
        }
    }
    if (merged.length === 0) {
        return [{ text: "", style: {} }];
    }
    return merged;
}

export function replaceBlockChunks(block: NoteBlock, chunks: TextChunk[]): NoteBlock {
    const normalized = normalizeChunks(chunks);
    const nextContent = getTextContent(block.content);
    if (block.content && typeof block.content === "object" && "Text" in block.content) {
        return {
            ...block,
            content: { Text: { chunks: normalized } },
        };
    }
    if (nextContent) {
        return {
            ...block,
            content: { text: { chunks: normalized } },
        };
    }
    return block;
}

function splitChunksAt(chunks: TextChunk[], offset: number): TextChunk[] {
    if (offset <= 0) {
        return cloneChunks(chunks);
    }
    const out: TextChunk[] = [];
    let pos = 0;
    for (const raw of chunks) {
        const chunk = cloneChunk(raw);
        const next = pos + scalarLen(chunk.text);
        if (offset <= pos || offset >= next) {
            out.push(chunk);
            pos = next;
            continue;
        }
        const local = offset - pos;
        const before = sliceByScalar(chunk.text, 0, local);
        const after = sliceByScalar(chunk.text, local, scalarLen(chunk.text));
        if (before.length > 0) {
            out.push({ text: before, style: normalizeStyle(chunk.style) });
        }
        if (after.length > 0) {
            out.push({ text: after, style: normalizeStyle(chunk.style) });
        }
        pos = next;
    }
    return normalizeChunks(out);
}

export function insertTextIntoChunks(
    chunks: TextChunk[],
    position: number,
    text: string,
    style: TextStyle,
): TextChunk[] {
    const normalized = normalizeChunks(chunks);
    const split = splitChunksAt(normalized, position);
    const out: TextChunk[] = [];
    let pos = 0;
    let inserted = false;
    for (const chunk of split) {
        if (!inserted && pos >= position) {
            out.push({ text, style: normalizeStyle(style) });
            inserted = true;
        }
        out.push(cloneChunk(chunk));
        pos += scalarLen(chunk.text);
    }
    if (!inserted) {
        out.push({ text, style: normalizeStyle(style) });
    }
    return normalizeChunks(out);
}

export function deleteRangeFromChunks(
    chunks: TextChunk[],
    start: number,
    end: number,
): TextChunk[] {
    if (end <= start) {
        return normalizeChunks(chunks);
    }
    const out: TextChunk[] = [];
    let pos = 0;
    for (const raw of chunks) {
        const chunk = cloneChunk(raw);
        const next = pos + scalarLen(chunk.text);
        if (next <= start || pos >= end) {
            out.push(chunk);
            pos = next;
            continue;
        }
        const localStart = Math.max(0, start - pos);
        const localEnd = Math.min(scalarLen(chunk.text), end - pos);
        const before = sliceByScalar(chunk.text, 0, localStart);
        const after = sliceByScalar(chunk.text, localEnd, scalarLen(chunk.text));
        if (before.length > 0) {
            out.push({ text: before, style: normalizeStyle(chunk.style) });
        }
        if (after.length > 0) {
            out.push({ text: after, style: normalizeStyle(chunk.style) });
        }
        pos = next;
    }
    return normalizeChunks(out);
}

export function setFormattingOnChunks(
    chunks: TextChunk[],
    start: number,
    end: number,
    style: TextStyle,
    enabled: boolean,
): TextChunk[] {
    if (end <= start) {
        return normalizeChunks(chunks);
    }
    const out: TextChunk[] = [];
    let pos = 0;
    for (const raw of chunks) {
        const chunk = cloneChunk(raw);
        const next = pos + scalarLen(chunk.text);
        if (next <= start || pos >= end) {
            out.push(chunk);
            pos = next;
            continue;
        }
        const localStart = Math.max(0, start - pos);
        const localEnd = Math.min(scalarLen(chunk.text), end - pos);
        const before = sliceByScalar(chunk.text, 0, localStart);
        const middle = sliceByScalar(chunk.text, localStart, localEnd);
        const after = sliceByScalar(chunk.text, localEnd, scalarLen(chunk.text));
        if (before.length > 0) {
            out.push({ text: before, style: normalizeStyle(chunk.style) });
        }
        if (middle.length > 0) {
            const nextStyle = normalizeStyle(chunk.style);
            if (style.bold === true) {
                nextStyle.bold = enabled;
            }
            if (style.italic === true) {
                nextStyle.italic = enabled;
            }
            if (style.color) {
                nextStyle.color = enabled ? style.color : null;
            }
            out.push({ text: middle, style: nextStyle });
        }
        if (after.length > 0) {
            out.push({ text: after, style: normalizeStyle(chunk.style) });
        }
        pos = next;
    }
    return normalizeChunks(out);
}
