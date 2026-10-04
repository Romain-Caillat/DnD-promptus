use std::env;
use std::path::PathBuf;

pub struct Config {
    pub database_url: String,
    pub port: u16,
    pub allowed_origins: Vec<String>,
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
            front_dir,
        })
    }
}
