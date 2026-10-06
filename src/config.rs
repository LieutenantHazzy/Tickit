use serde::Deserialize;
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub app_env: String,
    pub bind_addr: String,
    pub bind_port: u16,
    pub app_secret: String,
    pub base_url: String,
    pub default_lang: String,
    pub data_dir: PathBuf,
    pub db_path: PathBuf,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub mail_from: String,
    pub mail_to_errors: String,
    pub cookie_name: String,
    pub cookie_secure: bool,
    pub cookie_same_site: String,
    pub rate_limit_per_5min: i32,
    pub registration_open: bool,
    pub daily_reminder_hour: u32,
    pub daily_reminder_min: u32,
    pub reminder_offset_min: i32,
    pub log_level: String,
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        dotenvy::dotenv().ok();
        let cfg = config::Config::builder()
            .add_source(config::Environment::default())
            .build()?;
        Ok(cfg.try_deserialize()?)
    }
}
