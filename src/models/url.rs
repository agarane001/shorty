use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;

#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct UrlModel {
    pub short_code: String,
    pub long_url: String,
    pub user_id: Option<uuid::Uuid>,
    pub clicks: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
