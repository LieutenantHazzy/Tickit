use crate::config::Config;
use lettre::{
    message::header::ContentType, transport::smtp::authentication::Credentials, AsyncSmtpTransport,
    AsyncTransport, Message, Tokio1Executor,
};
use tracing::{error, warn};

#[derive(Clone)]
pub struct Mailer {
    transport: Option<AsyncSmtpTransport<Tokio1Executor>>,
    from: String,
    to_errors: String,
    enabled: bool,
}

impl Mailer {
    pub fn new(config: &Config) -> Self {
        if config.smtp_host.trim().is_empty() {
            warn!("SMTP_HOST not set, mail is disabled");
            return Self {
                transport: None,
                from: config.mail_from.clone(),
                to_errors: config.mail_to_errors.clone(),
                enabled: false,
            };
        }
        let creds = Credentials::new(config.smtp_username.clone(), config.smtp_password.clone());
        let transport = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.smtp_host)
            .unwrap_or_else(|_| {
                AsyncSmtpTransport::<Tokio1Executor>::relay(&config.smtp_host)
                    .expect("Failed to create SMTP transport")
            })
            .port(config.smtp_port)
            .credentials(creds)
            .build();
        Self {
            transport: Some(transport),
            from: config.mail_from.clone(),
            to_errors: config.mail_to_errors.clone(),
            enabled: true,
        }
    }

    pub async fn send_error(&self, subject: &str, body: &str) {
        if !self.enabled {
            return;
        }
        let Some(ref transport) = self.transport else {
            return;
        };
        let email = match Message::builder()
            .from(
                self.from
                    .parse()
                    .unwrap_or_else(|_| self.from.as_str().parse().unwrap()),
            )
            .to(self.to_errors.parse().unwrap())
            .subject(subject)
            .header(ContentType::TEXT_HTML)
            .body(body.to_string())
        {
            Ok(e) => e,
            Err(e) => {
                error!("Failed to build error email: {}", e);
                return;
            }
        };
        if let Err(e) = transport.send(email).await {
            error!("Failed to send error email: {}", e);
        }
    }

    pub async fn send_daily(&self, to: &str, subject: &str, body: &str) {
        if !self.enabled {
            return;
        }
        let Some(ref transport) = self.transport else {
            return;
        };
        let email = match Message::builder()
            .from(
                self.from
                    .parse()
                    .unwrap_or_else(|_| self.from.as_str().parse().unwrap()),
            )
            .to(to.parse().unwrap())
            .subject(subject)
            .header(ContentType::TEXT_HTML)
            .body(body.to_string())
        {
            Ok(e) => e,
            Err(e) => {
                error!("Failed to build email: {}", e);
                return;
            }
        };
        if let Err(e) = transport.send(email).await {
            error!("Failed to send email: {}", e);
        }
    }
}
