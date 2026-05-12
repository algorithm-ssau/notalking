CREATE TABLE users (
    id TEXT PRIMARY KEY NOT NULL,
    login TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE sessions (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device TEXT NOT NULL DEFAULT '',
    location TEXT NOT NULL DEFAULT '',
    issued_at TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    revoked_at TEXT
);

CREATE INDEX idx_sessions_user ON sessions(user_id);

CREATE TABLE notes (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    head_id TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX idx_notes_user ON notes(user_id);

CREATE TABLE note_documents (
    note_id TEXT PRIMARY KEY NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
    payload TEXT NOT NULL
);
