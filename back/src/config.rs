use std::env;
use std::path::PathBuf;

use crate::auth::mailer::SmtpConfig;

pub struct Config {
    pub database_url: String,
    pub port: u16,
    pub allowed_origins: Vec<String>,
    /// The origin people open the app at (`PUBLIC_ORIGIN`). The session
    /// cookie is `Secure` when it is HTTPS.
    pub public_origin: String,
    /// Where sign-in codes are sent from (`SMTP_*`). Unset in
    /// development: codes then go to the server log.
    pub smtp: Option<SmtpConfig>,
    /// The built front to serve next to the API (`FRONT_DIR`). Unset in
    /// development, where Vite serves the front and proxies `/api`.
    pub front_dir: Option<PathBuf>,
}

impl Config {
    /// Read the server configuration from the environment. `.env.example`
    /// at the repository root lists every variable.
    ///
    /// # Errors
    ///
    /// Fails when `DATABASE_URL` is missing, `PORT` is not a valid u16,
    /// or `FRONT_DIR` is set but holds no `index.html`.
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        let database_url =
            env::var("DATABASE_URL").map_err(|_| "DATABASE_URL must be set (see .env.example)")?;
        let port_str = env::var("PORT").unwrap_or_else(|_| "4333".to_string());
        let port: u16 = port_str
            .parse()
            .map_err(|_| format!("Invalid PORT value: '{port_str}' — must be a valid u16"))?;
        let allowed_origins = env::var("ALLOWED_ORIGINS")
            .unwrap_or_else(|_| {
                "http://localhost:4334,http://127.0.0.1:4334,tauri://localhost".to_string()
            })
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        let public_origin =
            non_empty("PUBLIC_ORIGIN").unwrap_or_else(|| "http://localhost:4334".to_string());
        let front_dir = match env::var("FRONT_DIR") {
            Ok(dir) if !dir.is_empty() => {
                let dir = PathBuf::from(dir);
                // Fail at start rather than answer every page with a 404.
                if !dir.join("index.html").is_file() {
                    return Err(format!("FRONT_DIR={} holds no index.html", dir.display()).into());
                }
                Some(dir)
            }
            _ => None,
        };
        Ok(Self {
            database_url,
            port,
            allowed_origins,
            public_origin,
            smtp: smtp_from_env()?,
            front_dir,
        })
    }
}

/// `SMTP_HOST` turns email on; `SMTP_FROM` is then required.
fn smtp_from_env() -> Result<Option<SmtpConfig>, String> {
    let Some(host) = non_empty("SMTP_HOST") else {
        return Ok(None);
    };
    let port_str = non_empty("SMTP_PORT").unwrap_or_else(|| "587".to_string());
    let port = port_str
        .parse()
        .map_err(|_| format!("Invalid SMTP_PORT value: '{port_str}' — must be a valid u16"))?;
    let from = non_empty("SMTP_FROM").ok_or("SMTP_FROM must be set when SMTP_HOST is")?;
    Ok(Some(SmtpConfig {
        host,
        port,
        user: non_empty("SMTP_USER"),
        password: env::var("SMTP_PASS").ok().filter(|v| !v.is_empty()),
        from,
    }))
}

fn non_empty(name: &str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}
