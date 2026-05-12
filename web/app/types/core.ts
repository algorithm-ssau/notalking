export interface SessionResponse {
    session_id: string;
    user_id: string;
    issued_at: string;
    expires_at: string;
}

export interface ManagedSessionResponse {
    session_id: string;
    device: string;
    location: string;
    issued_at: string;
    expires_at: string;
    updated_at: string;
    revoked_at: string | null;
    is_current: boolean;
}

export interface SessionsListResponse {
    sessions: ManagedSessionResponse[];
}

export interface CloseOtherSessionsResponse {
    closed_count: number;
}

export interface NoteResponse {
    id: string;
    title: string;
    head_id: string | null;
    created_at: string;
    updated_at: string;
}

export interface NotesListResponse {
    notes: NoteResponse[];
    page: number;
    per_page: number;
    total: number;
    total_pages: number;
}

export interface SemanticSearchRequest {
    query: string;
    limit?: number;
}

export interface SemanticHitResponse {
    note_id: string;
    block_id: string;
    score: number;
}

export interface SemanticSearchResponse {
    hits: SemanticHitResponse[];
}

export interface ListNotesParams {
    page?: number;
    per_page?: number;
}
