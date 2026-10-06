use crate::{
    error::AppError,
    handlers::auth::{extract_session, AuthState},
    models::list::List,
};
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{get, put},
    Json, Router,
};
use chrono::Utc;
use serde::Deserialize;

pub fn router() -> Router<AuthState> {
    Router::new()
        .route("/", get(list_lists).post(create_list))
        .route("/:id", put(update_list).delete(delete_list))
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

async fn list_lists(
    State(state): State<AuthState>,
    headers: HeaderMap,
) -> Result<Json<Vec<List>>, AppError> {
    let uid = get_user_id(&state, &headers).await?;
    let lists =
        sqlx::query_as::<_, List>("SELECT * FROM lists WHERE user_id = ? ORDER BY created_at")
            .bind(uid)
            .fetch_all(&state.pool)
            .await?;
    Ok(Json(lists))
}

#[derive(Deserialize)]
struct ListReq {
    name: String,
}

async fn create_list(
    State(state): State<AuthState>,
    headers: HeaderMap,
    Json(req): Json<ListReq>,
) -> Result<Json<List>, AppError> {
    let uid = get_user_id(&state, &headers).await?;
    let id = sqlx::query("INSERT INTO lists (user_id, name) VALUES (?, ?)")
        .bind(uid)
        .bind(&req.name)
        .execute(&state.pool)
        .await?
        .last_insert_rowid();
    let list = sqlx::query_as::<_, List>("SELECT * FROM lists WHERE id = ?")
        .bind(id)
        .fetch_one(&state.pool)
        .await?;
    Ok(Json(list))
}

async fn update_list(
    State(state): State<AuthState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(req): Json<ListReq>,
) -> Result<StatusCode, AppError> {
    let uid = get_user_id(&state, &headers).await?;
    sqlx::query(
        "UPDATE lists SET name = ?, updated_at = datetime('now') WHERE id = ? AND user_id = ?",
    )
    .bind(&req.name)
    .bind(id)
    .bind(uid)
    .execute(&state.pool)
    .await?;
    Ok(StatusCode::OK)
}

async fn delete_list(
    State(state): State<AuthState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let uid = get_user_id(&state, &headers).await?;
    sqlx::query("DELETE FROM lists WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(uid)
        .execute(&state.pool)
        .await?;
    Ok(StatusCode::OK)
}
