use crate::{
    auth::{
        password::{hash_password, verify_password},
        session::{check_rate_limit, RateLimitStore},
        totp::{generate_secret, generate_totp_url, verify_totp},
    },
    config::Config,
    error::AppError,
    models::user::User,
};
use axum::{
    extract::State,
    http::{HeaderMap, HeaderValue, StatusCode},
    routing::{get, patch, post},
    Json, Router,
};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone)]
pub struct AuthState {
    pub pool: sqlx::SqlitePool,
    pub config: Config,
    pub rate_limit: RateLimitStore,
}

#[derive(Deserialize)]
pub struct LoginReq {
    pub email: String,
    pub password: String,
    pub totp_code: Option<String>,
}

#[derive(Deserialize)]
pub struct RegisterReq {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct MeRes {
    pub id: i64,
    pub email: String,
    pub lang: String,
    pub daily_reminder_hour: u32,
    pub daily_reminder_min: u32,
    pub totp_enabled: bool,
}

#[derive(Serialize)]
pub struct SetupRes {
    pub secret: String,
    pub qr_url: String,
}

pub fn router() -> Router<AuthState> {
    Router::new()
        .route("/login", post(login))
        .route("/logout", post(logout))
        .route("/register", post(register))
        .route("/me", get(me))
        .route("/preferences", patch(update_prefs))
        .route("/setup-2fa", get(setup_2fa).post(enable_2fa))
        .route("/disable-2fa", post(disable_2fa))
}

pub fn extract_session(headers: &HeaderMap, cookie_name: &str) -> Option<String> {
    let cookie = headers.get("cookie")?.to_str().ok()?;
    for part in cookie.split(';') {
        let mut it = part.trim().splitn(2, '=');
        if let (Some(k), Some(v)) = (it.next(), it.next()) {
            if k == cookie_name {
                return Some(v.to_string());
            }
        }
    }
    None
}

fn set_cookie_headers(
    headers: &mut HeaderMap,
    name: &str,
    value: &str,
    max_age: i64,
    secure: bool,
) {
    let mut s = String::from(name);
    s.push('=');
    s.push_str(value);
    s.push_str("; Path=/; HttpOnly");
    if max_age > 0 {
        s.push_str("; Max-Age=");
        s.push_str(&max_age.to_string());
    }
    if secure {
        s.push_str("; Secure");
    }
    s.push_str("; SameSite=Lax");
    headers.insert("set-cookie", HeaderValue::from_str(&s).unwrap());
}

async fn login(
    State(state): State<AuthState>,
    headers: HeaderMap,
    Json(req): Json<LoginReq>,
) -> Result<(HeaderMap, StatusCode), AppError> {
    let ip = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .unwrap_or("unknown");
    let key = format!("login:{}", ip);
    if !check_rate_limit(
        &state.rate_limit,
        &key,
        state.config.rate_limit_per_5min,
        300,
    ) {
        return Err(AppError::RateLimited);
    }
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = ? COLLATE NOCASE")
        .bind(&req.email)
        .fetch_optional(&state.pool)
        .await?;
    let Some(user) = user else {
        return Err(AppError::Unauthorized);
    };
    if !verify_password(&req.password, &user.password_hash) {
        return Err(AppError::Unauthorized);
    }
    if user.totp_enabled == 1 {
        let code = req
            .totp_code
            .ok_or_else(|| AppError::BadRequest("TOTP required".into()))?;
        if !verify_totp(user.totp_secret.as_deref().unwrap_or(""), &code) {
            return Err(AppError::Unauthorized);
        }
    }
    let session_id = Uuid::new_v4().to_string();
    let expires_at = Utc::now() + Duration::days(7);
    let ua = headers.get("user-agent").and_then(|v| v.to_str().ok());
    sqlx::query(
        "INSERT INTO sessions (id, user_id, expires_at, user_agent, ip) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&session_id)
    .bind(user.id)
    .bind(expires_at.to_rfc3339())
    .bind(ua)
    .bind(&ip)
    .execute(&state.pool)
    .await?;
    let mut headers = HeaderMap::new();
    set_cookie_headers(
        &mut headers,
        &state.config.cookie_name,
        &session_id,
        604800,
        state.config.cookie_secure,
    );
    Ok((headers, StatusCode::OK))
}

async fn register(
    State(state): State<AuthState>,
    headers: HeaderMap,
    Json(req): Json<RegisterReq>,
) -> Result<(HeaderMap, StatusCode), AppError> {
    let ip = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .unwrap_or("unknown");
    let key = format!("register:{}", ip);
    if !check_rate_limit(
        &state.rate_limit,
        &key,
        state.config.rate_limit_per_5min,
        300,
    ) {
        return Err(AppError::RateLimited);
    }
    if req.email.len() > 320 || req.password.len() < 12 || req.password.len() > 256 {
        return Err(AppError::Validation("password must be 12-256 chars".into()));
    }
    if !state.config.registration_open {
        return Err(AppError::Forbidden);
    }
    let existing = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = ? COLLATE NOCASE")
        .bind(&req.email)
        .fetch_optional(&state.pool)
        .await?;
    if existing.is_some() {
        return Err(AppError::Validation("email already registered".into()));
    }
    let hash = hash_password(&req.password)?;
    let id = sqlx::query("INSERT INTO users (email, password_hash, lang, daily_reminder_hour, daily_reminder_min) VALUES (?, ?, ?, ?, ?)")
        .bind(&req.email)
        .bind(&hash)
        .bind(state.config.default_lang.clone())
        .bind(state.config.daily_reminder_hour)
        .bind(state.config.daily_reminder_min)
        .execute(&state.pool)
        .await?
        .last_insert_rowid();
    let session_id = Uuid::new_v4().to_string();
    let expires_at = Utc::now() + Duration::days(7);
    let ua = headers.get("user-agent").and_then(|v| v.to_str().ok());
    sqlx::query(
        "INSERT INTO sessions (id, user_id, expires_at, user_agent, ip) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&session_id)
    .bind(id)
    .bind(expires_at.to_rfc3339())
    .bind(ua)
    .bind(&ip)
    .execute(&state.pool)
    .await?;
    let mut headers = HeaderMap::new();
    set_cookie_headers(
        &mut headers,
        &state.config.cookie_name,
        &session_id,
        604800,
        state.config.cookie_secure,
    );
    Ok((headers, StatusCode::OK))
}

async fn logout(State(state): State<AuthState>, headers: HeaderMap) -> (HeaderMap, StatusCode) {
    if let Some(sid) = extract_session(&headers, &state.config.cookie_name) {
        sqlx::query("DELETE FROM sessions WHERE id = ?")
            .bind(&sid)
            .execute(&state.pool)
            .await
            .ok();
    }
    let mut headers = HeaderMap::new();
    set_cookie_headers(
        &mut headers,
        &state.config.cookie_name,
        "",
        0,
        state.config.cookie_secure,
    );
    (headers, StatusCode::OK)
}

pub async fn get_user_id(state: &AuthState, headers: &HeaderMap) -> Result<i64, AppError> {
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

async fn get_user(state: &AuthState, headers: &HeaderMap) -> Result<User, AppError> {
    let uid = get_user_id(state, headers).await?;
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
        .bind(uid)
        .fetch_one(&state.pool)
        .await?;
    Ok(user)
}

async fn me(State(state): State<AuthState>, headers: HeaderMap) -> Result<Json<MeRes>, AppError> {
    let user = get_user(&state, &headers).await?;
    Ok(Json(MeRes {
        id: user.id,
        email: user.email,
        lang: user.lang,
        daily_reminder_hour: user.daily_reminder_hour,
        daily_reminder_min: user.daily_reminder_min,
        totp_enabled: user.totp_enabled == 1,
    }))
}

#[derive(Deserialize)]
struct PrefsReq {
    lang: Option<String>,
    daily_reminder_hour: Option<u32>,
    daily_reminder_min: Option<u32>,
}

async fn update_prefs(
    State(state): State<AuthState>,
    headers: HeaderMap,
    Json(req): Json<PrefsReq>,
) -> Result<StatusCode, AppError> {
    let uid = get_user_id(&state, &headers).await?;
    if let Some(lang) = req.lang {
        if !matches!(lang.as_str(), "en" | "nl") {
            return Err(AppError::Validation("lang must be en or nl".into()));
        }
        sqlx::query("UPDATE users SET lang = ? WHERE id = ?")
            .bind(lang)
            .bind(uid)
            .execute(&state.pool)
            .await?;
    }
    if let Some(h) = req.daily_reminder_hour {
        if let Some(m) = req.daily_reminder_min {
            if h > 23 || m > 59 {
                return Err(AppError::Validation("invalid time".into()));
            }
            sqlx::query(
                "UPDATE users SET daily_reminder_hour = ?, daily_reminder_min = ? WHERE id = ?",
            )
            .bind(h)
            .bind(m)
            .bind(uid)
            .execute(&state.pool)
            .await?;
        }
    }
    Ok(StatusCode::OK)
}

async fn setup_2fa(
    State(state): State<AuthState>,
    headers: HeaderMap,
) -> Result<Json<SetupRes>, AppError> {
    let user = get_user(&state, &headers).await?;
    let secret = if let Some(existing) = &user.totp_secret {
        existing.clone()
    } else {
        let s = generate_secret();
        sqlx::query("UPDATE users SET totp_secret = ? WHERE id = ?")
            .bind(&s)
            .bind(user.id)
            .execute(&state.pool)
            .await?;
        s
    };
    let qr = generate_totp_url(&secret, &user.email, "Tickit")?;
    Ok(Json(SetupRes { secret, qr_url: qr }))
}

#[derive(Deserialize)]
struct EnableReq {
    code: String,
}

async fn enable_2fa(
    State(state): State<AuthState>,
    headers: HeaderMap,
    Json(req): Json<EnableReq>,
) -> Result<StatusCode, AppError> {
    let user = get_user(&state, &headers).await?;
    let secret = user
        .totp_secret
        .as_deref()
        .ok_or_else(|| AppError::BadRequest("No secret".into()))?;
    if !verify_totp(secret, &req.code) {
        return Err(AppError::Unauthorized);
    }
    sqlx::query("UPDATE users SET totp_enabled = 1 WHERE id = ?")
        .bind(user.id)
        .execute(&state.pool)
        .await?;
    Ok(StatusCode::OK)
}

async fn disable_2fa(
    State(state): State<AuthState>,
    headers: HeaderMap,
) -> Result<StatusCode, AppError> {
    let user = get_user(&state, &headers).await?;
    sqlx::query("UPDATE users SET totp_enabled = 0, totp_secret = NULL WHERE id = ?")
        .bind(user.id)
        .execute(&state.pool)
        .await?;
    Ok(StatusCode::OK)
}
