const DEFAULT_INTELLIGENCE_BASE_URL = "/intel";
export const SUPER_PROMPT_STORAGE_KEY = "notalking:agent-super-prompt";
export const SELECTED_PROVIDER_STORAGE_KEY = "notalking:selected-intel-provider";
export const ASSISTANT_PREFERENCES_CHANGED_EVENT = "notalking:assistant-preferences-changed";

function normalizeBaseUrl(baseUrl: string): string {
    return baseUrl.endsWith("/") ? baseUrl.slice(0, -1) : baseUrl;
}

function dispatchAssistantPreferencesChanged(key: string, value: string) {
    if (!import.meta.client) {
        return;
    }
    window.dispatchEvent(
        new CustomEvent(ASSISTANT_PREFERENCES_CHANGED_EVENT, {
            detail: { key, value },
        }),
    );
}

export function readSelectedIntelProviderId(): string {
    if (!import.meta.client) {
        return "";
    }
    return (localStorage.getItem(SELECTED_PROVIDER_STORAGE_KEY) || "").trim();
}

export function writeSelectedIntelProviderId(providerId: string): void {
    if (!import.meta.client) {
        return;
    }
    const value = providerId.trim();
    if (value) {
        localStorage.setItem(SELECTED_PROVIDER_STORAGE_KEY, value);
    } else {
        localStorage.removeItem(SELECTED_PROVIDER_STORAGE_KEY);
    }
    dispatchAssistantPreferencesChanged(SELECTED_PROVIDER_STORAGE_KEY, value);
}

export function readSuperPrompt(): string {
    if (!import.meta.client) {
        return "";
    }
    return localStorage.getItem(SUPER_PROMPT_STORAGE_KEY) || "";
}

export function writeSuperPrompt(prompt: string): void {
    if (!import.meta.client) {
        return;
    }
    const value = prompt.replace(/\r\n/g, "\n");
    if (value.trim()) {
        localStorage.setItem(SUPER_PROMPT_STORAGE_KEY, value);
    } else {
        localStorage.removeItem(SUPER_PROMPT_STORAGE_KEY);
    }
    dispatchAssistantPreferencesChanged(SUPER_PROMPT_STORAGE_KEY, value);
}

export function useIntelApi() {
    const config = useRuntimeConfig();
    const baseUrl = computed(() => normalizeBaseUrl((config.public.intelligenceApiUrl as string) || DEFAULT_INTELLIGENCE_BASE_URL));

    return {
        streamIntelChat: (body: IntelChatRequest, onEvent: (ev: IntelStreamEvent) => void) =>
            streamIntelChatFromBase(baseUrl.value, body, onEvent),
        fetchIntelProviderCatalog: () => fetchIntelProviderCatalogFromBase(baseUrl.value),
        fetchIntelProviders: () => fetchIntelProvidersFromBase(baseUrl.value),
        createIntelProvider: (body: IntelProviderCreate) => createIntelProviderFromBase(baseUrl.value, body),
        deleteIntelProvider: (providerId: string) => deleteIntelProviderFromBase(baseUrl.value, providerId),
    };
}

function formatFastApiDetail(detail: unknown): string {
    if (detail == null) {
        return "";
    }
    if (typeof detail === "string") {
        return detail;
    }
    if (typeof detail === "object" && detail !== null && !Array.isArray(detail)) {
        const d = detail as Record<string, unknown>;
        if (typeof d.message === "string") {
            const preview = typeof d.core_response_preview === "string" && d.core_response_preview.trim()
                ? ` Preview: ${d.core_response_preview.trim()}`
                : "";
            const attempts = Array.isArray(d.attempts) && d.attempts.length
                ? ` Attempts: ${d.attempts.map((a) => {
                    if (!a || typeof a !== "object") {
                        return "";
                    }
                    const item = a as Record<string, unknown>;
                    const url = typeof item.url === "string" ? item.url : "unknown";
                    const status = item.status != null ? ` -> ${String(item.status)}` : "";
                    const error = typeof item.error === "string" ? ` -> ${item.error}` : "";
                    return `${url}${status}${error}`;
                }).filter(Boolean).join("; ")}`
                : "";
            return `${d.message}${preview}${attempts}`;
        }
        if (typeof d.msg === "string") {
            return d.msg;
        }
    }
    return JSON.stringify(detail);
}

export type IntelChatMessage = { role: "system" | "user" | "assistant"; content: string };

export type IntelProvider = {
    id: string;
    kind: string;
    display_name: string;
    config: Record<string, unknown>;
    created_at: string;
    updated_at: string;
};

export type IntelProviderCatalogEntry = {
    kind: string;
    label: string;
    description?: string;
    config_fields?: Record<string, unknown>;
};

export type IntelProviderCreate = {
    kind: string;
    display_name: string;
    config?: Record<string, unknown>;
};

export type IntelChatRequest = {
    messages: IntelChatMessage[];
    note_id?: string | null;
    provider_id?: string | null;
    super_prompt?: string | null;
};

export type IntelToolPhase = "start" | "done" | "error";

export type IntelToolNote = {
    note_id?: string;
    title?: string;
    matched_by?: string;
    score?: number;
    excerpt?: string;
    block_id?: string;
};

export type IntelToolPayload = Record<string, unknown>;

export type IntelToolEvent = {
    type: "tool";
    source?: "core_bridge" | string;
    name: string;
    phase: IntelToolPhase;
    call_id?: string;
    method?: string;
    mcp_method?: string;
    message?: string;
    query?: string;
    limit?: number;
    count?: number;
    note_id?: string;
    title?: string;
    block_count?: number;
    matched_by?: string;
    reason?: string;
    error?: string;
    notes?: IntelToolNote[];
    request?: IntelToolPayload;
    minimal_response?: IntelToolPayload;
};

export type IntelStreamEvent =
    | { type: "start"; stream_id?: string }
    | IntelToolEvent
    | {
        type: "action";
        action?: "note_created" | "note_create_failed" | string;
        message?: string;
        note_id?: string;
        title?: string;
        head_block_id?: string;
      }
    | { type: "token"; text: string }
    | { type: "done" }
    | { type: "error"; message?: string; status?: number };

function parseSseBlocks(buffer: string): { events: IntelStreamEvent[]; rest: string } {
    const events: IntelStreamEvent[] = [];
    const parts = buffer.split("\n\n");
    const rest = parts.pop() ?? "";
    for (const block of parts) {
        const line = block.trim().split("\n").find((l) => l.startsWith("data:"));
        if (!line) {
            continue;
        }
        const jsonPart = line.slice(5).trim();
        if (!jsonPart || jsonPart === "[DONE]") {
            continue;
        }
        try {
            events.push(JSON.parse(jsonPart) as IntelStreamEvent);
        } catch {
            /* ignore malformed chunk */
        }
    }
    return { events, rest };
}

/**
 * POST Intelligence SSE chat; invokes onEvent for each parsed event (tokens and errors).
 */
export async function streamIntelChat(
    body: IntelChatRequest,
    onEvent: (ev: IntelStreamEvent) => void,
): Promise<void> {
    return streamIntelChatFromBase(DEFAULT_INTELLIGENCE_BASE_URL, body, onEvent);
}

async function streamIntelChatFromBase(
    baseUrl: string,
    body: IntelChatRequest,
    onEvent: (ev: IntelStreamEvent) => void,
): Promise<void> {
    const res = await fetch(`${normalizeBaseUrl(baseUrl)}/chat/completions/stream`, {
        method: "POST",
        credentials: "include",
        headers: {
            "Content-Type": "application/json",
            Accept: "text/event-stream",
        },
        body: JSON.stringify({
            messages: body.messages,
            note_id: body.note_id || undefined,
            provider_id: body.provider_id || undefined,
            super_prompt: body.super_prompt?.trim() || undefined,
        }),
    });

    if (!res.ok) {
        let detail = res.statusText;
        try {
            const errBody = (await res.json()) as { detail?: unknown };
            detail = formatFastApiDetail(errBody.detail ?? errBody);
        } catch {
            /* ignore */
        }
        throw new Error(`Chat request failed (${res.status}): ${detail}`);
    }

    const reader = res.body?.getReader();
    if (!reader) {
        throw new Error("No response body");
    }

    const decoder = new TextDecoder();
    let buf = "";

    while (true) {
        const { done, value } = await reader.read();
        if (done) {
            break;
        }
        buf += decoder.decode(value, { stream: true });
        const { events, rest } = parseSseBlocks(buf);
        buf = rest;
        for (const ev of events) {
            onEvent(ev);
        }
    }

    if (buf.trim()) {
        const { events } = parseSseBlocks(buf + "\n\n");
        for (const ev of events) {
            onEvent(ev);
        }
    }
}

async function readIntelError(res: Response): Promise<string> {
    let detail = res.statusText;
    try {
        const errBody = (await res.json()) as { detail?: unknown };
        detail = formatFastApiDetail(errBody.detail ?? errBody);
    } catch {
        /* ignore */
    }
    return detail;
}

export async function fetchIntelProviderCatalog(): Promise<IntelProviderCatalogEntry[]> {
    return fetchIntelProviderCatalogFromBase(DEFAULT_INTELLIGENCE_BASE_URL);
}

async function fetchIntelProviderCatalogFromBase(baseUrl: string): Promise<IntelProviderCatalogEntry[]> {
    const res = await fetch(`${normalizeBaseUrl(baseUrl)}/providers/catalog`, { credentials: "include" });
    if (!res.ok) {
        throw new Error(`Could not load provider catalog (${res.status}): ${await readIntelError(res)}`);
    }
    const rows = (await res.json()) as IntelProviderCatalogEntry[];
    return Array.isArray(rows) ? rows : [];
}

export async function fetchIntelProviders(): Promise<IntelProvider[]> {
    return fetchIntelProvidersFromBase(DEFAULT_INTELLIGENCE_BASE_URL);
}

async function fetchIntelProvidersFromBase(baseUrl: string): Promise<IntelProvider[]> {
    const res = await fetch(`${normalizeBaseUrl(baseUrl)}/providers`, { credentials: "include" });
    if (!res.ok) {
        throw new Error(`Could not load providers (${res.status}): ${await readIntelError(res)}`);
    }
    const rows = (await res.json()) as IntelProvider[];
    return Array.isArray(rows) ? rows : [];
}

export async function createIntelProvider(body: IntelProviderCreate): Promise<IntelProvider> {
    return createIntelProviderFromBase(DEFAULT_INTELLIGENCE_BASE_URL, body);
}

async function createIntelProviderFromBase(baseUrl: string, body: IntelProviderCreate): Promise<IntelProvider> {
    const res = await fetch(`${normalizeBaseUrl(baseUrl)}/providers`, {
        method: "POST",
        credentials: "include",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
            kind: body.kind,
            display_name: body.display_name,
            config: body.config ?? {},
        }),
    });
    if (!res.ok) {
        throw new Error(`Could not create provider (${res.status}): ${await readIntelError(res)}`);
    }
    return (await res.json()) as IntelProvider;
}

export async function deleteIntelProvider(providerId: string): Promise<void> {
    return deleteIntelProviderFromBase(DEFAULT_INTELLIGENCE_BASE_URL, providerId);
}

async function deleteIntelProviderFromBase(baseUrl: string, providerId: string): Promise<void> {
    const res = await fetch(`${normalizeBaseUrl(baseUrl)}/providers/${encodeURIComponent(providerId)}`, {
        method: "DELETE",
        credentials: "include",
    });
    if (!res.ok) {
        throw new Error(`Could not delete provider (${res.status}): ${await readIntelError(res)}`);
    }
}
