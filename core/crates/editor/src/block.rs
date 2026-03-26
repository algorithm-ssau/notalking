use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::content::Content;

pub struct Block<M = ()> {
    pub id: Uuid,
    pub prev_id: Option<Uuid>,
    pub next_id: Option<Uuid>,
    pub content: Content,
    pub metadata: M,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
