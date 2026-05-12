# Intelligence relational DDL reference

The Intelligence service creates tables at startup via SQLAlchemy (`intel_providers`) against **`INTELLIGENCE_DATABASE_URL`**.

For operational migrations outside application bootstrap, maintain Alembic or hand-written SQL aligned with [`src/notalking_intelligence/db/models.py`](../src/notalking_intelligence/db/models.py). Example SQLite shape:

```sql
CREATE TABLE IF NOT EXISTS intel_providers (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    display_name TEXT NOT NULL,
    config_json TEXT NOT NULL DEFAULT '{}',
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS ix_intel_providers_user_id ON intel_providers (user_id);
```
