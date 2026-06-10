//! NAPP backend — orchestratielaag rond de regelrecht-engine.
//!
//! Axum API met sessie-gebaseerde auth: echte SSO Rijk (OIDC) voor
//! beoordelaars wanneer geconfigureerd, gemockte eHerkenning voor aanvragers.

mod beheer;
mod claim;
mod db;
mod engine;
mod handelsregister;
mod handlers;
mod machtiging;
mod register;
mod rekening;
mod state;

use std::sync::Arc;

use axum::routing::{get, post};
use axum::Router;
use regelrecht_auth::OidcAppState;
use sqlx::sqlite::SqlitePoolOptions;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;
use tower_sessions::{Expiry, SessionManagerLayer};
use tower_sessions_memory_store::MemoryStore;

use state::{AppState, LawCorpus};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "napp_backend=info,tower_http=info".into()),
        )
        .init();

    let corpus = Arc::new(LawCorpus::load()?);
    tracing::info!("wetscorpus geladen (Wpp, regeling, AWB)");

    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:napp.db?mode=rwc".to_string());
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
    db::init(&pool).await?;
    register::seed_if_empty(&pool).await?;
    tracing::info!(database = %database_url, "database gereed");

    // OIDC (SSO Rijk) — alleen actief wanneer de OIDC_* variabelen gezet zijn.
    let oidc_config = regelrecht_auth::parse_oidc_from_env()
        .map_err(|e| anyhow::anyhow!("OIDC-configuratie ongeldig: {e}"))?;
    let (oidc_client, end_session_url) = if let Some(ref config) = oidc_config {
        match regelrecht_auth::discover_client(config).await {
            Ok(result) => {
                tracing::info!("SSO Rijk (OIDC) actief");
                (Some(Arc::new(result.client)), result.end_session_url)
            }
            Err(e) => {
                tracing::error!(error = %e, "OIDC-discovery mislukt");
                return Err(anyhow::anyhow!("OIDC-discovery mislukt: {e}"));
            }
        }
    } else {
        tracing::warn!("OIDC niet geconfigureerd — mock-SSO-login actief (alleen voor demo)");
        (None, None)
    };

    let app_state = AppState {
        pool,
        corpus,
        oidc_client,
        oidc_config,
        end_session_url,
        base_url: std::env::var("BASE_URL").ok(),
        http_client: reqwest::Client::new(),
    };

    let auth_routes = regelrecht_auth::auth_routes::<AppState>();

    let mut api = Router::new()
        .route("/api/me", get(handlers::me))
        .route("/api/eherkenning/login", post(handlers::eherkenning_login))
        .route(
            "/api/eherkenning/logout",
            post(handlers::eherkenning_logout),
        )
        .route(
            "/api/eherkenning/machtigingen",
            get(machtiging::machtigingen),
        )
        .route("/api/mijn-registratie", get(handlers::mijn_registratie))
        .route(
            "/api/mijn-rekening",
            get(rekening::get_mijn_rekening).put(rekening::put_mijn_rekening),
        )
        // Claim-flow: een rechtspersoon koppelt zichzelf aan een
        // ONGEKOPPELDE aanduiding uit de uitslag (zie claim.rs).
        .route("/api/claim/aanduidingen", get(claim::list_aanduidingen))
        .route("/api/claim", post(claim::create_claim))
        .route("/api/mijn-claim", get(claim::mijn_claim))
        .route("/api/register/demo", get(handlers::register_demo))
        .route("/api/aanvragen", post(handlers::create_aanvraag))
        .route("/api/aanvragen/proef", post(handlers::proef_aanspraken))
        .route("/api/aanvragen", get(handlers::list_aanvragen))
        .route("/api/mijn-aanvragen", get(handlers::list_mijn_aanvragen))
        .route("/api/mijn-aanvragen/{id}", get(handlers::get_mijn_aanvraag))
        .route("/api/aanvragen/{id}", get(handlers::get_aanvraag))
        .route(
            "/api/aanvragen/{id}/proefberekening",
            post(handlers::proefberekening),
        )
        .route(
            "/api/aanvragen/{id}/besluit",
            post(handlers::stel_besluit_vast),
        )
        .route(
            "/api/aanvragen/{id}/bekendmaking",
            post(handlers::bekendmaking),
        )
        .route(
            "/api/betaalopdrachten",
            get(handlers::list_betaalopdrachten),
        )
        .route("/api/register", get(handlers::register))
        .route("/api/register/statistieken", get(handlers::statistieken))
        // Partijregister-beheer (beoordelaar-only, zie beheer.rs). De
        // uitslagen zijn referentiedata (Kiesraad/CBS) en kennen bewust
        // geen mutatie-endpoints; koppelingen ontstaan via de claim-flow.
        .route("/api/beheer/partijen", get(beheer::list_partijen))
        .route(
            "/api/beheer/partijen/{kvk}",
            get(beheer::get_partij).put(beheer::update_partij),
        )
        .route("/api/beheer/claims", get(claim::beheer_list_claims))
        .route(
            "/api/beheer/claims/{id}/bevestig",
            post(claim::bevestig_claim),
        )
        .route(
            "/api/beheer/claims/{id}/afwijzen",
            post(claim::wijs_claim_af),
        );

    // Mock-SSO-login alleen registreren wanneer echte OIDC uit staat.
    if !app_state.is_auth_enabled() {
        api = api.route("/api/sso/mock-login", post(handlers::sso_mock_login));
    }

    let session_layer = SessionManagerLayer::new(MemoryStore::default())
        .with_expiry(Expiry::OnInactivity(time::Duration::hours(8)))
        .with_same_site(tower_sessions::cookie::SameSite::Lax)
        .with_http_only(true)
        .with_secure(false);

    let static_dir = std::env::var("NAPP_STATIC_DIR").unwrap_or_else(|_| "frontend/dist".into());
    let index_file = format!("{static_dir}/index.html");

    let app = Router::new()
        .route("/health", get(handlers::health))
        .merge(auth_routes)
        .merge(api)
        .with_state(app_state)
        .layer(session_layer)
        .layer(TraceLayer::new_for_http())
        .fallback_service(ServeDir::new(&static_dir).not_found_service(ServeFile::new(&index_file)));

    let port: u16 = std::env::var("NAPP_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8400);
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await?;
    tracing::info!("NAPP-backend luistert op http://localhost:{port}");
    axum::serve(listener, app).await?;
    Ok(())
}
