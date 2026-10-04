use promptus_back::auth::setup::SetupState;
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

    let router = app::router(AppState { pool, auth }, &config.allowed_origins);

    let addr = format!("0.0.0.0:{}", config.port);
    tracing::info!("Server listening on {addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, router).await?;
    Ok(())
}
