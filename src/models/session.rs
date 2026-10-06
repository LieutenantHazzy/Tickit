use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Session {
    pub id: String,
    pub user_id: i64,
    pub expires_at: String,
    pub user_agent: Option<String>,
    pub ip: Option<String>,
    pub created_at: String,
}
