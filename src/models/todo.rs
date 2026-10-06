use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Todo {
    pub id: i64,
    pub user_id: i64,
    pub list_id: Option<i64>,
    pub title: String,
    pub description_md: Option<String>,
    pub done: i32,
    pub due_date: Option<String>,
    pub due_time: Option<String>,
    pub remind_at_utc: Option<String>,
    pub reminded_at: Option<String>,
    pub priority: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct TodoCreate {
    pub list_id: Option<i64>,
    pub title: String,
    pub description_md: Option<String>,
    pub due_date: Option<String>,
    pub due_time: Option<String>,
    pub priority: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct TodoUpdate {
    pub list_id: Option<i64>,
    pub title: Option<String>,
    pub description_md: Option<String>,
    pub done: Option<i32>,
    pub due_date: Option<String>,
    pub due_time: Option<String>,
    pub priority: Option<i32>,
}
