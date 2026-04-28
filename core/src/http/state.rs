use std::sync::Arc;

use crate::{
    auth::{
        password::ArgonPasswordService,
        service::AuthService,
    },
    config::CoreConfig,
    embedding::EmbeddingRuntime,
    http::rate_limit::RateLimiterHandle,
    note::service::NoteService,
    persist::{SqlNoteStore, SqlSessionRepo, SqlUserRepo},
};

#[derive(Clone)]
pub struct AppState {
    pub auth: AuthService<SqlUserRepo, SqlSessionRepo, ArgonPasswordService>,
    pub note_store: Arc<SqlNoteStore>,
    pub note: NoteService<SqlNoteStore, SqlNoteStore>,
    pub rate_limiter: RateLimiterHandle,
    pub config: Arc<CoreConfig>,
    pub embedding: Option<Arc<EmbeddingRuntime>>,
}

impl AppState {
    pub async fn build(db: Arc<crate::db::Db>, config: Arc<CoreConfig>) -> anyhow::Result<Self> {
        let note_store = Arc::new(SqlNoteStore::new(db.clone()));
        let embedding = match EmbeddingRuntime::try_new(&config, note_store.clone()).await {
            Ok(e) => e.map(Arc::new),
            Err(e) => {
                tracing::warn!(error = %e, "embedding runtime disabled");
                None
            }
        };
        let rate_limiter = RateLimiterHandle::new_async(&config).await;

        let user_repo = Arc::new(SqlUserRepo::new(db.clone()));
        let session_repo = Arc::new(SqlSessionRepo::new(db));
        let password_service = Arc::new(ArgonPasswordService);

        Ok(Self {
            auth: AuthService::new(user_repo, session_repo, password_service),
            note_store: note_store.clone(),
            note: NoteService::new(note_store.clone(), note_store),
            rate_limiter,
            config,
            embedding,
        })
    }

    pub fn notify_embedding(&self, user_id: uuid::Uuid, note_id: uuid::Uuid) {
        if let Some(ref e) = self.embedding {
            e.notify_blocks_changed(user_id, note_id);
        }
    }
}
