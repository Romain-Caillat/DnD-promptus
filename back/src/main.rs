use promptus_back::auth::setup::SetupState;
use promptus_back::live::{self, LiveConfig, LiveHub};
use promptus_back::state::{AppState, Auth};
use promptus_back::{app, config::Config, db};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let config = Config::from_env()?;

    let pool = db::connect(&config.database_url).await?;
    db::migrate(&pool).await?;
    tracing::info!("Database migrations applied successfully");

    let setup = SetupState::init(&pool, config.gm_setup_token.clone())
        .await
        .map_err(|e| format!("setup state: {e:?}"))?;
    if let Some(code) = setup.code() {
        // The only place the code appears: whoever reads the server log
        // runs the server, and may create the first GM account.
        tracing::warn!(
            "No GM account yet. Create the first one at {}/inscription with the setup code: {code}",
            config.public_origin
        );
    }
    let auth = Auth::new(
        &config.public_origin,
        config.webauthn_rp_id.as_deref(),
        setup,
    )
    .map_err(|e| {
        format!("passkeys: {e}. Set PUBLIC_ORIGIN (and WEBAUTHN_RP_ID if needed), see .env.example")
    })?;

    let live = LiveHub::new(LiveConfig::default());
    live::listener::spawn(pool.clone(), live.clone());
    promptus_back::evening::schedule::spawn(pool.clone());

    let mut router = app::router(
        AppState {
            pool,
            auth,
            live,
            ai: promptus_back::ai::Ai::from_env(),
        },
        &config.allowed_origins,
    );
    if let Some(dir) = &config.front_dir {
        tracing::info!("Serving the front from {}", dir.display());
        router = app::with_front(router, dir);
    }

    let addr = format!("0.0.0.0:{}", config.port);
    tracing::info!("Server listening on {addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

/// Resolve on Ctrl-C or SIGTERM. In a container the server is PID 1,
/// which the kernel shields from default signal actions: without this
/// `docker stop` would wait its full timeout and then kill it.
async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut sig) => {
                sig.recv().await;
            }
            Err(_) => std::future::pending::<()>().await,
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
    tracing::info!("Shutting down");
}
