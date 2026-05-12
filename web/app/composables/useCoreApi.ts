import type { NoteBlock } from "~/types/editor";
import type {
    CloseOtherSessionsResponse,
    ListNotesParams,
    NoteResponse,
    NotesListResponse,
    SemanticSearchRequest,
    SemanticSearchResponse,
    SessionResponse,
    SessionsListResponse,
} from "~/types/core";

const CORE = "/core";

const fetchOpts = { credentials: "include" as const };

function notesQuery(params?: ListNotesParams): string {
    if (!params?.page && !params?.per_page) {
        return "";
    }
    const q = new URLSearchParams();
    if (params.page != null) {
        q.set("page", String(params.page));
    }
    if (params.per_page != null) {
        q.set("per_page", String(params.per_page));
    }
    const s = q.toString();
    return s ? `?${s}` : "";
}

export function useCoreApi() {
    return {
        register(login: string, password: string) {
            return $fetch<SessionResponse>(`${CORE}/auth/register`, {
                ...fetchOpts,
                method: "POST",
                body: { login, password },
            });
        },

        login(login: string, password: string) {
            return $fetch<SessionResponse>(`${CORE}/auth/login`, {
                ...fetchOpts,
                method: "POST",
                body: { login, password },
            });
        },

        logout() {
            return $fetch(`${CORE}/auth/logout`, {
                ...fetchOpts,
                method: "POST",
            });
        },

        listSessions() {
            return $fetch<SessionsListResponse>(`${CORE}/auth/sessions`, fetchOpts);
        },

        closeSession(sessionId: string) {
            return $fetch(`${CORE}/auth/sessions/${encodeURIComponent(sessionId)}`, {
                ...fetchOpts,
                method: "DELETE",
            });
        },

        closeOtherSessions() {
            return $fetch<CloseOtherSessionsResponse>(
                `${CORE}/auth/sessions/others`,
                {
                    ...fetchOpts,
                    method: "DELETE",
                },
            );
        },

        listNotes(params?: ListNotesParams) {
            return $fetch<NotesListResponse>(
                `${CORE}/notes${notesQuery(params)}`,
                fetchOpts,
            );
        },

        createNote(title: string, body = "") {
            return $fetch<NoteResponse>(`${CORE}/notes`, {
                ...fetchOpts,
                method: "POST",
                body: { title, body },
            });
        },

        deleteNote(noteId: string) {
            return $fetch(`${CORE}/notes/${encodeURIComponent(noteId)}`, {
                ...fetchOpts,
                method: "DELETE",
            });
        },

        listBlocks(noteId: string) {
            return $fetch<{ blocks: NoteBlock[] }>(
                `${CORE}/notes/${encodeURIComponent(noteId)}/blocks`,
                fetchOpts,
            );
        },

        createTextBlock(noteId: string, afterId: string | null, text = "") {
            return $fetch<NoteBlock>(`${CORE}/notes/${encodeURIComponent(noteId)}/blocks`, {
                ...fetchOpts,
                method: "POST",
                body: {
                    type: "text",
                    after_id: afterId,
                    text,
                },
            });
        },

        patchBlock(
            noteId: string,
            blockId: string,
            body: Record<string, unknown>,
        ) {
            return $fetch(
                `${CORE}/notes/${encodeURIComponent(noteId)}/blocks/${encodeURIComponent(blockId)}`,
                {
                    ...fetchOpts,
                    method: "PATCH",
                    body,
                },
            );
        },

        deleteBlock(noteId: string, blockId: string) {
            return $fetch(
                `${CORE}/notes/${encodeURIComponent(noteId)}/blocks/${encodeURIComponent(blockId)}`,
                {
                    ...fetchOpts,
                    method: "DELETE",
                },
            );
        },

        semanticSearch(body: SemanticSearchRequest) {
            return $fetch<SemanticSearchResponse>(`${CORE}/search/semantic`, {
                ...fetchOpts,
                method: "POST",
                body: {
                    query: body.query,
                    ...(body.limit != null ? { limit: body.limit } : {}),
                },
            });
        },
    };
}
