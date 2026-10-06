use crate::{
    error::AppError,
    handlers::auth::{extract_session, AuthState},
    models::todo::{Todo, TodoCreate, TodoUpdate},
};
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post, put},
    Json, Router,
};
use chrono::{Local, NaiveDate, NaiveTime, TimeZone, Utc};
use serde::Serialize;

pub fn router() -> Router<AuthState> {
    Router::new()
        .route("/", get(list_todos).post(create_todo))
        .route("/:id", put(update_todo).delete(delete_todo))
        .route("/:id/toggle", post(toggle_todo))
        .route("/export/json", get(export_json))
}

async fn get_user_id(state: &AuthState, headers: &HeaderMap) -> Result<i64, AppError> {
    let sid = extract_session(headers, &state.config.cookie_name).ok_or(AppError::Unauthorized)?;
    let session =
        sqlx::query_as::<_, crate::models::session::Session>("SELECT * FROM sessions WHERE id = ?")
            .bind(&sid)
            .fetch_optional(&state.pool)
            .await?;
    let session = session.ok_or(AppError::Unauthorized)?;
    if chrono::DateTime::parse_from_rfc3339(&session.expires_at)
        .unwrap()
        .timestamp()
        < Utc::now().timestamp()
    {
        return Err(AppError::Unauthorized);
    }
    Ok(session.user_id)
}

async fn list_todos(
    State(state): State<AuthState>,
    headers: HeaderMap,
) -> Result<Json<Vec<Todo>>, AppError> {
    let uid = get_user_id(&state, &headers).await?;
    let todos =
        sqlx::query_as::<_, Todo>("SELECT * FROM todos WHERE user_id = ? ORDER BY created_at DESC")
            .bind(uid)
            .fetch_all(&state.pool)
            .await?;
    Ok(Json(todos))
}

fn calc_remind_at(due_date: &str, due_time: &str, offset_min: i32) -> Option<String> {
    if let Ok(date) = NaiveDate::parse_from_str(due_date, "%Y-%m-%d") {
        if let Ok(time) = NaiveTime::parse_from_str(due_time, "%H:%M") {
            let naive = date.and_time(time);
            let local = Local.from_local_datetime(&naive).single()?;
            let remind = local.with_timezone(&Utc) - chrono::Duration::minutes(offset_min as i64);
            return Some(remind.to_rfc3339());
        }
    }
    None
}

async fn create_todo(
    State(state): State<AuthState>,
    headers: HeaderMap,
    Json(req): Json<TodoCreate>,
) -> Result<Json<Todo>, AppError> {
    let uid = get_user_id(&state, &headers).await?;
    let remind_at_utc = match (&req.due_date, &req.due_time) {
        (Some(d), Some(t)) => calc_remind_at(d, t, state.config.reminder_offset_min),
        _ => None,
    };
    let id = sqlx::query(
        "INSERT INTO todos (user_id, list_id, title, description_md, due_date, due_time, remind_at_utc, priority) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(uid)
    .bind(req.list_id)
    .bind(&req.title)
    .bind(&req.description_md)
    .bind(&req.due_date)
    .bind(&req.due_time)
    .bind(&remind_at_utc)
    .bind(req.priority.unwrap_or(0))
    .execute(&state.pool)
    .await?
    .last_insert_rowid();
    let todo = sqlx::query_as::<_, Todo>("SELECT * FROM todos WHERE id = ?")
        .bind(id)
        .fetch_one(&state.pool)
        .await?;
    Ok(Json(todo))
}

async fn update_todo(
    State(state): State<AuthState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(req): Json<TodoUpdate>,
) -> Result<StatusCode, AppError> {
    let uid = get_user_id(&state, &headers).await?;
    let todo = sqlx::query_as::<_, Todo>("SELECT * FROM todos WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(uid)
        .fetch_optional(&state.pool)
        .await?;
    let Some(todo) = todo else {
        return Err(AppError::NotFound);
    };
    let title = req.title.as_ref().unwrap_or(&todo.title);
    let desc = req.description_md.as_ref().or(todo.description_md.as_ref());
    let list_id = req.list_id.or(todo.list_id);
    let due_date = req.due_date.as_ref().or(todo.due_date.as_ref());
    let due_time = req.due_time.as_ref().or(todo.due_time.as_ref());
    let priority = req.priority.unwrap_or(todo.priority);
    let remind_at_utc = match (due_date, due_time) {
        (Some(d), Some(t)) => calc_remind_at(d, t, state.config.reminder_offset_min),
        _ => None,
    };
    sqlx::query(
        "UPDATE todos SET list_id = ?, title = ?, description_md = ?, done = ?, due_date = ?, due_time = ?, remind_at_utc = ?, priority = ?, updated_at = datetime('now') WHERE id = ? AND user_id = ?",
    )
    .bind(list_id)
    .bind(title)
    .bind(desc)
    .bind(req.done.unwrap_or(todo.done))
    .bind(due_date)
    .bind(due_time)
    .bind(&remind_at_utc)
    .bind(priority)
    .bind(id)
    .bind(uid)
    .execute(&state.pool)
    .await?;
    Ok(StatusCode::OK)
}

async fn toggle_todo(
    State(state): State<AuthState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let uid = get_user_id(&state, &headers).await?;
    sqlx::query("UPDATE todos SET done = 1 - done, updated_at = datetime('now') WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(uid)
        .execute(&state.pool)
        .await?;
    Ok(StatusCode::OK)
}

async fn delete_todo(
    State(state): State<AuthState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let uid = get_user_id(&state, &headers).await?;
    sqlx::query("DELETE FROM todos WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(uid)
        .execute(&state.pool)
        .await?;
    Ok(StatusCode::OK)
}

#[derive(Serialize)]
struct Export {
    user_id: i64,
    todos: Vec<Todo>,
    lists: Vec<crate::models::list::List>,
}

async fn export_json(
    State(state): State<AuthState>,
    headers: HeaderMap,
) -> Result<Json<Export>, AppError> {
    let uid = get_user_id(&state, &headers).await?;
    let todos = sqlx::query_as::<_, Todo>("SELECT * FROM todos WHERE user_id = ?")
        .bind(uid)
        .fetch_all(&state.pool)
        .await?;
    let lists =
        sqlx::query_as::<_, crate::models::list::List>("SELECT * FROM lists WHERE user_id = ?")
            .bind(uid)
            .fetch_all(&state.pool)
            .await?;
    Ok(Json(Export {
        user_id: uid,
        todos,
        lists,
    }))
}
