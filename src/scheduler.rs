use crate::{config::Config, mailer::Mailer, models::todo::Todo, models::user::User};
use chrono::{Local, Timelike, Utc};
use sqlx::SqlitePool;
use tokio_cron_scheduler::{Job, JobScheduler};

pub async fn start_scheduler(
    pool: SqlitePool,
    config: Config,
    mailer: Mailer,
) -> anyhow::Result<JobScheduler> {
    let scheduler = JobScheduler::new().await?;
    let p = pool.clone();
    let c = config.clone();
    let m = mailer.clone();
    let minute_job = Job::new_async("0 * * * * *", move |_uuid, _l| {
        let pool = p.clone();
        let config = c.clone();
        let mailer = m.clone();
        Box::pin(async move {
            if let Err(e) = send_daily_reminders(pool.clone(), config.clone(), mailer.clone()).await
            {
                tracing::error!("Daily reminder failed: {}", e);
            }
            if let Err(e) = send_reminders_15min(pool, config, mailer).await {
                tracing::error!("Reminder failed: {}", e);
            }
        })
    })?;
    scheduler.add(minute_job).await?;
    scheduler.start().await?;
    Ok(scheduler)
}

async fn send_daily_reminders(
    pool: SqlitePool,
    _config: Config,
    mailer: Mailer,
) -> anyhow::Result<()> {
    let now = Local::now();
    let today = now.format("%Y-%m-%d").to_string();
    let hour = now.hour();
    let min = now.minute();
    let users = sqlx::query_as::<_, User>("SELECT * FROM users")
        .fetch_all(&pool)
        .await?;
    for user in users {
        if user.daily_reminder_hour as u32 != hour || user.daily_reminder_min as u32 != min {
            continue;
        }
        let todos = sqlx::query_as::<_, Todo>(
            "SELECT * FROM todos WHERE user_id = ? AND due_date = ? AND done = 0 ORDER BY due_time",
        )
        .bind(user.id)
        .bind(&today)
        .fetch_all(&pool)
        .await?;
        if todos.is_empty() {
            continue;
        }
        let mut body = String::from("<h2>Daily reminders for today</h2><ul>");
        for t in &todos {
            let time_str = t.due_time.as_deref().unwrap_or("");
            body.push_str(&format!("<li>{} {}</li>", t.title, time_str));
        }
        body.push_str("</ul>");
        mailer
            .send_daily(&user.email, "Tickit - Today's todos", &body)
            .await;
    }
    Ok(())
}

async fn send_reminders_15min(
    pool: SqlitePool,
    _config: Config,
    mailer: Mailer,
) -> anyhow::Result<()> {
    let now = Utc::now();
    let todos = sqlx::query_as::<_, Todo>(
        "SELECT * FROM todos WHERE remind_at_utc IS NOT NULL AND reminded_at IS NULL AND done = 0",
    )
    .fetch_all(&pool)
    .await?;
    for t in todos {
        if let Some(ref remind) = t.remind_at_utc {
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(remind) {
                if dt <= now {
                    if let Ok(user) = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
                        .bind(t.user_id)
                        .fetch_one(&pool)
                        .await
                    {
                        let body = format!(
                            "<h2>Reminder</h2><p>{}</p><p>Due: {} {}</p>",
                            t.title,
                            t.due_date.as_deref().unwrap_or(""),
                            t.due_time.as_deref().unwrap_or("")
                        );
                        mailer
                            .send_daily(&user.email, "Tickit - Reminder", &body)
                            .await;
                    }
                    sqlx::query("UPDATE todos SET reminded_at = ? WHERE id = ?")
                        .bind(now.to_rfc3339())
                        .bind(t.id)
                        .execute(&pool)
                        .await?;
                }
            }
        }
    }
    Ok(())
}
