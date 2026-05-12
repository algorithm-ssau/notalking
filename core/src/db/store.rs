use std::path::{Path, PathBuf};

use sqlx::migrate::Migrator;
use sqlx::postgres::PgPool;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool};

pub enum Db {
    Sqlite(SqlitePool),
    Postgres(PgPool),
}

impl Db {
    /// For plain `sqlite:relative/path` URLs, the file path is resolved against `crate_root`
    /// so `cargo run` works from any working directory (matches migrations under `core/`).
    pub async fn connect(database_url: &str, crate_root: &Path) -> Result<Self, sqlx::Error> {
        if database_url.starts_with("postgres://") || database_url.starts_with("postgresql://") {
            let pool = PgPool::connect(database_url).await?;
            return Ok(Db::Postgres(pool));
        }

        if database_url.starts_with("sqlite://") || database_url.starts_with("sqlite::memory:") {
            let pool = SqlitePool::connect(database_url).await?;
            return Ok(Db::Sqlite(pool));
        }

        let rest = database_url.strip_prefix("sqlite:").unwrap_or(database_url);
        if rest == ":memory:" {
            let pool = SqlitePool::connect("sqlite::memory:").await?;
            return Ok(Db::Sqlite(pool));
        }

        let p = Path::new(rest);
        let path: PathBuf = if p.is_relative() {
            crate_root.join(p)
        } else {
            p.to_path_buf()
        };
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let opts = SqliteConnectOptions::new()
            .filename(&path)
            .create_if_missing(true);
        let pool = SqlitePool::connect_with(opts).await?;
        Ok(Db::Sqlite(pool))
    }

    pub async fn run_migrations_from_crate_root(
        &self,
        crate_dir: &std::path::Path,
    ) -> Result<(), sqlx::migrate::MigrateError> {
        let sqlite = crate_dir.join("migrations/sqlite");
        let postgres = crate_dir.join("migrations/postgres");
        match self {
            Db::Sqlite(pool) => {
                let m = Migrator::new(sqlite.as_path()).await?;
                m.run(pool).await
            }
            Db::Postgres(pool) => {
                let m = Migrator::new(postgres.as_path()).await?;
                m.run(pool).await
            }
        }
    }
}
