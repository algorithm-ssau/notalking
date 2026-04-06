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
