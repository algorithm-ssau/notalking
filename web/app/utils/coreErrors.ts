type FetchErrorShape = {
    data?: { code?: string; message?: string; error?: string };
    message?: string;
    statusCode?: number;
    statusMessage?: string;
    cause?: { message?: string };
};

/**
 * Human-readable message from a failed Core `$fetch` call. Core returns JSON `{ code, message }` (see `ApiErrorBody`).
 */
export function getCoreErrorMessage(
    err: unknown,
    fallback: string,
): string {
    const e = err as FetchErrorShape;
    const fromBody =
        e?.data?.message ?? e?.data?.error ?? e?.message ?? e?.cause?.message;
    if (fromBody) {
        if (/fetch failed|ECONNREFUSED|connect/i.test(fromBody)) {
            return "Could not reach Core. Start the Core service and check the /core proxy.";
        }
        return fromBody;
    }
    const code = e?.statusCode;
    if (code === 401) {
        return "Session expired or not signed in. Sign in again.";
    }
    if (code === 503) {
        return "This feature is not available (service unavailable).";
    }
    if (code === 502 || code === 503) {
        return "Core is unavailable. Start the service and check the /core proxy.";
    }
    if (code === 429) {
        return "Too many requests. Try again later.";
    }
    return fallback;
}

export function isEmbeddingsDisabledError(err: unknown): boolean {
    const e = err as FetchErrorShape;
    return e?.data?.code === "embeddings_disabled" || e?.statusCode === 503;
}
