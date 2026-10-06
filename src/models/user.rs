use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: i64,
    pub email: String,
    pub password_hash: String,
    pub totp_secret: Option<String>,
    pub totp_enabled: i32,
    pub lang: String,
    pub daily_reminder_hour: u32,
    pub daily_reminder_min: u32,
    pub created_at: String,
    pub updated_at: String,
}
