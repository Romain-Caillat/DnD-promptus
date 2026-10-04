use std::env;

pub struct Config {
    pub database_url: String,
    pub port: u16,
    pub allowed_origins: Vec<String>,
    /// The origin people open the app at (`PUBLIC_ORIGIN`). Passkeys
    /// are bound to it, and the session cookie is `Secure` when it is
    /// HTTPS.
    pub public_origin: String,
    /// `WEBAUTHN_RP_ID`: the domain passkeys belong to. Defaults to the
    /// host of `public_origin`.
    pub webauthn_rp_id: Option<String>,
    /// `GM_SETUP_TOKEN`: pins the first-account setup code instead of a
    /// random one printed at boot.
    pub gm_setup_token: Option<String>,
}

impl Config {
    /// Read the server configuration from the environment. `.env.example`
    /// at the repository root lists every variable.
    ///
    /// # Errors
    ///
    /// Fails when `DATABASE_URL` is missing or `PORT` is not a valid u16.
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
        Ok(Self {
            database_url,
            port,
            allowed_origins,
            public_origin,
            webauthn_rp_id: non_empty("WEBAUTHN_RP_ID"),
            gm_setup_token: non_empty("GM_SETUP_TOKEN"),
        })
    }
}

fn non_empty(name: &str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}
