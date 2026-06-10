//! Application state shared across handlers.

use std::path::PathBuf;
use std::sync::Arc;

use regelrecht_auth::{ConfiguredClient, OidcAppState, OidcConfig};
use sqlx::SqlitePool;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub corpus: Arc<LawCorpus>,
    pub oidc_client: Option<Arc<ConfiguredClient>>,
    pub oidc_config: Option<OidcConfig>,
    pub end_session_url: Option<String>,
    pub base_url: Option<String>,
    pub http_client: reqwest::Client,
}

impl OidcAppState for AppState {
    fn oidc_client(&self) -> Option<&Arc<ConfiguredClient>> {
        self.oidc_client.as_ref()
    }
    fn end_session_url(&self) -> Option<&str> {
        self.end_session_url.as_deref()
    }
    fn oidc_config(&self) -> Option<&OidcConfig> {
        self.oidc_config.as_ref()
    }
    fn is_auth_enabled(&self) -> bool {
        self.oidc_config.is_some()
    }
    fn base_url(&self) -> Option<&str> {
        self.base_url.as_deref()
    }
    fn http_client(&self) -> &reqwest::Client {
        &self.http_client
    }
}

/// The law corpus: YAML sources loaded once at startup. The engine itself is
/// constructed per evaluation (cheap, and keeps the state Send + Sync).
pub struct LawCorpus {
    pub wpp: String,
    pub regeling: String,
    pub besluit_decentraal: String,
    pub awb: String,
    pub termijnenwet: String,
    /// Subset van de Kieswet (artikel G 1): de registratie-eisen voor een
    /// aanduiding, gebruikt bij de claim-toets in het partijregister.
    pub kieswet: String,
}

impl LawCorpus {
    pub fn load() -> anyhow::Result<Self> {
        let dir = std::env::var("NAPP_LAW_DIR").unwrap_or_else(|_| "law".to_string());
        let dir = PathBuf::from(dir);
        let read = |rel: &str| -> anyhow::Result<String> {
            let path = dir.join(rel);
            std::fs::read_to_string(&path)
                .map_err(|e| anyhow::anyhow!("kan wettekst {} niet lezen: {e}", path.display()))
        };
        Ok(Self {
            wpp: read("wet_op_de_politieke_partijen/2026-01-01.yaml")?,
            regeling: read("regeling_subsidiebedragen/2026-01-01.yaml")?,
            besluit_decentraal: read(
                "besluit_subsidiering_decentrale_politieke_partijen/2026-01-01.yaml",
            )?,
            awb: read("algemene_wet_bestuursrecht/1994-01-01.yaml")?,
            termijnenwet: read("algemene_termijnenwet/1964-04-01.yaml")?,
            kieswet: read("kieswet/1989-09-28.yaml")?,
        })
    }
}
