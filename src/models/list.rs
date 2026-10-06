use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct List {
    pub id: i64,
    pub user_id: i64,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
}
