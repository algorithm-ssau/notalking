use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::content::Content;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Block<M = ()> {
    pub id: Uuid,
    pub prev_id: Option<Uuid>,
    pub next_id: Option<Uuid>,
    pub content: Content,
    #[serde(default)]
    pub metadata: M,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
