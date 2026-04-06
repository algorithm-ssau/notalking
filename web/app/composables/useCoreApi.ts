import type { NoteBlock, NoteSummary } from "~/types/editor";

const CORE = "/core";

export function useCoreApi() {
    const fetchOpts = { credentials: "include" as const };

    return {
        register(login: string, password: string) {
            return $fetch<{ session_id: string }>(`${CORE}/auth/register`, {
                ...fetchOpts,
                method: "POST",
                body: { login, password },
            });
        },

        login(login: string, password: string) {
            return $fetch<{ session_id: string }>(`${CORE}/auth/login`, {
                ...fetchOpts,
                method: "POST",
                body: { login, password },
            });
        },

        listNotes() {
            return $fetch<{ notes: NoteSummary[] }>(`${CORE}/notes`, fetchOpts);
        },

        createNote(title: string, body = "") {
            return $fetch<NoteSummary>(`${CORE}/notes`, {
                ...fetchOpts,
                method: "POST",
                body: { title, body },
            });
        },

        listBlocks(noteId: string) {
            return $fetch<{ blocks: NoteBlock[] }>(`${CORE}/notes/${noteId}/blocks`, fetchOpts);
        },

        createTextBlock(noteId: string, afterId: string | null, text = "") {
            return $fetch<NoteBlock>(`${CORE}/notes/${noteId}/blocks`, {
                ...fetchOpts,
                method: "POST",
                body: {
                    type: "text",
                    after_id: afterId,
                    text,
                },
            });
        },

        patchBlock(noteId: string, blockId: string, body: Record<string, unknown>) {
            return $fetch(`${CORE}/notes/${noteId}/blocks/${blockId}`, {
                ...fetchOpts,
                method: "PATCH",
                body,
            });
        },
    };
}
