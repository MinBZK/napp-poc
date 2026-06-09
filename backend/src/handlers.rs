//! HTTP handlers: auth (mock eHerkenning + mock SSO fallback), aanvragen,
//! besluiten, bekendmaking, register en betaalopdrachten.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use regelrecht_auth::{
    SESSION_KEY_AUTHENTICATED, SESSION_KEY_EMAIL, SESSION_KEY_NAME, SESSION_KEY_SUB,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tower_sessions::Session;
use uuid::Uuid;

use crate::db;
use crate::engine;
use crate::state::AppState;

const SESSION_KEY_EH_KVK: &str = "eh_kvk";
const SESSION_KEY_EH_PARTIJ: &str = "eh_partij";

type ApiError = (StatusCode, Json<serde_json::Value>);

fn internal_error(e: impl std::fmt::Display) -> ApiError {
    tracing::error!(error = %e, "interne fout");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({"fout": "Er is een interne fout opgetreden."})),
    )
}

fn bad_request(msg: &str) -> ApiError {
    (StatusCode::BAD_REQUEST, Json(json!({"fout": msg})))
}

fn not_found() -> ApiError {
    (
        StatusCode::NOT_FOUND,
        Json(json!({"fout": "Niet gevonden."})),
    )
}

fn forbidden() -> ApiError {
    (
        StatusCode::FORBIDDEN,
        Json(json!({"fout": "Geen toegang."})),
    )
}

async fn session_kvk(session: &Session) -> Option<String> {
    session.get(SESSION_KEY_EH_KVK).await.ok().flatten()
}

async fn session_beoordelaar(session: &Session) -> Option<String> {
    let authenticated: bool = session
        .get(SESSION_KEY_AUTHENTICATED)
        .await
        .ok()
        .flatten()
        .unwrap_or(false);
    if !authenticated {
        return None;
    }
    let name: Option<String> = session.get(SESSION_KEY_NAME).await.ok().flatten();
    Some(name.unwrap_or_else(|| "beoordelaar".to_string()))
}

// ---------------------------------------------------------------------------
// Auth
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct EherkenningLogin {
    pub kvk_nummer: String,
}

/// MOCK eHerkenning-login voor aanvragers. eHerkenning levert alleen de
/// identiteit van de rechtspersoon (KvK); de partijnaam volgt uit het
/// partijregister van de Napp. Een onbekend KvK-nummer kan gewoon inloggen
/// en aanvragen — de wet wijst dan af (AWB 4:1: iedereen mag aanvragen).
pub async fn eherkenning_login(
    session: Session,
    Json(body): Json<EherkenningLogin>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let kvk = body.kvk_nummer.trim().to_string();
    if !kvk.chars().all(|c| c.is_ascii_digit()) || kvk.len() != 8 {
        return Err(bad_request("Vul een geldig KVK-nummer in (8 cijfers)."));
    }
    let naam = match crate::register::partij_by_kvk(&kvk) {
        Some(partij) => partij.naam.to_string(),
        None => format!("Organisatie {kvk}"),
    };
    session
        .insert(SESSION_KEY_EH_KVK, kvk.clone())
        .await
        .map_err(internal_error)?;
    session
        .insert(SESSION_KEY_EH_PARTIJ, naam.clone())
        .await
        .map_err(internal_error)?;
    Ok(Json(json!({
        "rol": "aanvrager",
        "kvk_nummer": kvk,
        "partij_naam": naam,
        "mock": true,
    })))
}

/// Aanspraken van de ingelogde rechtspersoon volgens het partijregister,
/// met per onderdeel de beschikbaarheid voor het subsidiejaar. De aanvraag
/// volgt de rechtspersoon: een centraal georganiseerde partij ziet hier al
/// haar landelijke en decentrale aanspraken; een afdeling met eigen
/// rechtspersoon alleen de hare.
pub async fn mijn_registratie(
    State(state): State<AppState>,
    session: Session,
) -> Result<Json<serde_json::Value>, ApiError> {
    let Some(kvk) = session_kvk(&session).await else {
        return Err(forbidden());
    };
    let jaar = chrono::Utc::now().format("%Y").to_string().parse::<i64>().unwrap_or(2026);
    let partij = crate::register::partij_by_kvk(&kvk);
    let bezet = db::bezette_componenten(&state.pool, &kvk, jaar)
        .await
        .map_err(internal_error)?;

    let componenten = aanspraken_voor(&kvk);
    let aanspraken: Vec<serde_json::Value> = componenten
        .iter()
        .map(|c| {
            let status = bezet
                .get(&c.key)
                .cloned()
                .unwrap_or_else(|| "BESCHIKBAAR".to_string());
            let mut v = serde_json::to_value(c).unwrap_or_default();
            v["status"] = json!(status);
            v
        })
        .collect();

    Ok(Json(json!({
        "partij": partij.map(|p| json!({
            "naam": p.naam,
            "kamerzetels": p.kamerzetels,
            "organisatiemodel": p.organisatiemodel,
        })),
        "subsidiejaar": jaar,
        "aanspraken": aanspraken,
    })))
}

/// Bouw de componenten (aanspraken) van een rechtspersoon uit het register.
/// Een onbekende organisatie krijgt één landelijke component met nul zetels:
/// de aanvraagroute blijft open (AWB 4:1), de wet wijst af.
fn aanspraken_voor(kvk: &str) -> Vec<db::Component> {
    let Some(partij) = crate::register::partij_by_kvk(kvk) else {
        return vec![db::Component {
            key: "LANDELIJK".into(),
            soort: "LANDELIJK".into(),
            orgaan: None,
            gebied_code: None,
            gebied: None,
            zetels: 0,
            inwoneraantal: 0,
        }];
    };
    let mut componenten = Vec::new();
    // Landelijke component alleen voor partijen met kamerzetels; een
    // afdeling of lokale partij heeft die aanspraak niet.
    if partij.kamerzetels > 0 || partij.decentrale_uitslagen.is_empty() {
        componenten.push(db::Component {
            key: "LANDELIJK".into(),
            soort: "LANDELIJK".into(),
            orgaan: None,
            gebied_code: None,
            gebied: None,
            zetels: partij.kamerzetels,
            inwoneraantal: 0,
        });
    }
    for u in &partij.decentrale_uitslagen {
        let gebied = crate::register::gebied_by_code(&u.gebied_code);
        componenten.push(db::Component {
            key: format!("{}:{}", u.orgaan, u.gebied_code),
            soort: "DECENTRAAL".into(),
            orgaan: Some(u.orgaan.clone()),
            gebied_code: Some(u.gebied_code.clone()),
            gebied: gebied.map(|g| g.naam.clone()),
            zetels: u.zetels,
            inwoneraantal: gebied.map(|g| g.inwoneraantal).unwrap_or(0),
        });
    }
    componenten
}

/// Demo-voorbeelden voor de gemockte eHerkenning-login (alleen metadata).
pub async fn register_demo() -> Json<serde_json::Value> {
    Json(json!(crate::register::register().demo_voorbeelden))
}

pub async fn eherkenning_logout(session: Session) -> Result<Json<serde_json::Value>, ApiError> {
    session
        .remove::<String>(SESSION_KEY_EH_KVK)
        .await
        .map_err(internal_error)?;
    session
        .remove::<String>(SESSION_KEY_EH_PARTIJ)
        .await
        .map_err(internal_error)?;
    Ok(Json(json!({"uitgelogd": true})))
}

#[derive(Deserialize)]
pub struct MockSsoLogin {
    pub naam: String,
}

/// MOCK SSO-login voor beoordelaars — alleen actief wanneer OIDC niet is
/// geconfigureerd (lokale ontwikkeling). Zet dezelfde sessiesleutels als de
/// echte regelrecht-auth callback, zodat /auth/status gewoon werkt.
pub async fn sso_mock_login(
    session: Session,
    Json(body): Json<MockSsoLogin>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let naam = body.naam.trim();
    if naam.is_empty() {
        return Err(bad_request("Naam is verplicht."));
    }
    session
        .insert(SESSION_KEY_AUTHENTICATED, true)
        .await
        .map_err(internal_error)?;
    session
        .insert(SESSION_KEY_NAME, naam.to_string())
        .await
        .map_err(internal_error)?;
    session
        .insert(SESSION_KEY_EMAIL, format!("{}@napp.nl", naam.replace(' ', ".").to_lowercase()))
        .await
        .map_err(internal_error)?;
    session
        .insert(SESSION_KEY_SUB, format!("mock-{naam}"))
        .await
        .map_err(internal_error)?;
    Ok(Json(json!({"rol": "beoordelaar", "naam": naam, "mock": true})))
}

/// Gecombineerde sessie-status voor de frontend: welke rol(len) zijn actief.
pub async fn me(session: Session) -> Result<Json<serde_json::Value>, ApiError> {
    let kvk = session_kvk(&session).await;
    let partij: Option<String> = session.get(SESSION_KEY_EH_PARTIJ).await.ok().flatten();
    let beoordelaar = session_beoordelaar(&session).await;
    Ok(Json(json!({
        "aanvrager": kvk.map(|k| json!({"kvk_nummer": k, "partij_naam": partij})),
        "beoordelaar": beoordelaar.map(|n| json!({"naam": n})),
    })))
}

// ---------------------------------------------------------------------------
// Aanvragen (aanvrager)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct NieuweAanvraag {
    /// Sleutels van de aan te vragen onderdelen ("LANDELIJK" of "{orgaan}:{code}").
    pub componenten: Vec<String>,
    /// Eigen opgaven: aantal_betalende_leden en de vier transparantieverklaringen.
    pub parameters: serde_json::Map<String, serde_json::Value>,
}

pub async fn create_aanvraag(
    State(state): State<AppState>,
    session: Session,
    Json(body): Json<NieuweAanvraag>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let Some(kvk) = session_kvk(&session).await else {
        return Err(forbidden());
    };
    let partij: String = session
        .get(SESSION_KEY_EH_PARTIJ)
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "Onbekende organisatie".to_string());
    let jaar = chrono::Utc::now().format("%Y").to_string().parse::<i64>().unwrap_or(2026);

    if body.componenten.is_empty() {
        return Err(bad_request("Kies ten minste één onderdeel om aan te vragen."));
    }

    // De componenten komen uit het register, nooit uit de client: de client
    // stuurt alleen sleutels; gegevens (zetels, inwoneraantallen) worden hier
    // bevroren op registerwaarden.
    let beschikbaar = aanspraken_voor(&kvk);
    let mut componenten: Vec<db::Component> = Vec::new();
    for key in &body.componenten {
        let Some(c) = beschikbaar.iter().find(|c| &c.key == key) else {
            return Err(bad_request(&format!(
                "Onderdeel '{key}' hoort niet bij uw registratie."
            )));
        };
        componenten.push(c.clone());
    }

    // Dubbeldetectie: onderdelen die dit jaar al lopen of zijn toegekend.
    let bezet = db::bezette_componenten(&state.pool, &kvk, jaar)
        .await
        .map_err(internal_error)?;
    if let Some(c) = componenten.iter().find(|c| bezet.contains_key(&c.key)) {
        return Err(bad_request(&format!(
            "Onderdeel '{}' is voor {jaar} al aangevraagd of toegekend.",
            c.key
        )));
    }

    // Eigen opgaven: verklaringen verplicht, ledental optioneel (alleen
    // relevant voor de landelijke component).
    let mut eigen = serde_json::Map::new();
    for key in [
        "ontvangt_anonieme_giften",
        "ontvangt_giften_niet_ingezetenen",
        "voldoet_aan_meldplicht_giften",
        "financien_openbaar_op_website",
    ] {
        let Some(waarde) = body.parameters.get(key).and_then(|v| v.as_bool()) else {
            return Err(bad_request(&format!("Verplichte verklaring '{key}' ontbreekt.")));
        };
        eigen.insert(key.to_string(), json!(waarde));
    }
    let leden = body
        .parameters
        .get("aantal_betalende_leden")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    eigen.insert("aantal_betalende_leden".to_string(), json!(leden));

    let id = Uuid::new_v4().to_string();
    let vandaag = Utc::now().format("%Y-%m-%d").to_string();
    db::insert_aanvraag(
        &state.pool,
        &id,
        &kvk,
        &partij,
        jaar,
        &serde_json::to_string(&componenten).map_err(internal_error)?,
        &serde_json::to_string(&eigen).map_err(internal_error)?,
        &vandaag,
    )
    .await
    .map_err(internal_error)?;

    Ok(Json(json!({"id": id, "status": "BEHANDELING", "subsidiejaar": jaar})))
}

pub async fn list_aanvragen(
    State(state): State<AppState>,
    session: Session,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Beoordelaar ziet alles; aanvrager alleen de eigen aanvragen.
    let beoordelaar = session_beoordelaar(&session).await;
    let kvk = session_kvk(&session).await;
    let aanvragen = match (&beoordelaar, &kvk) {
        (Some(_), _) => db::list_aanvragen(&state.pool, None).await,
        (None, Some(kvk)) => db::list_aanvragen(&state.pool, Some(kvk)).await,
        (None, None) => return Err(forbidden()),
    }
    .map_err(internal_error)?;

    let mut result = Vec::with_capacity(aanvragen.len());
    for aanvraag in aanvragen {
        let besluit = db::get_besluit_by_aanvraag(&state.pool, &aanvraag.id)
            .await
            .map_err(internal_error)?;
        result.push(json!({"aanvraag": aanvraag, "besluit": besluit}));
    }
    Ok(Json(json!(result)))
}

pub async fn get_aanvraag(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let beoordelaar = session_beoordelaar(&session).await;
    let kvk = session_kvk(&session).await;
    let Some(aanvraag) = db::get_aanvraag(&state.pool, &id).await.map_err(internal_error)? else {
        return Err(not_found());
    };
    let allowed = beoordelaar.is_some() || kvk.as_deref() == Some(aanvraag.kvk_nummer.as_str());
    if !allowed {
        return Err(forbidden());
    }
    let besluit = db::get_besluit_by_aanvraag(&state.pool, &id)
        .await
        .map_err(internal_error)?;
    Ok(Json(json!({"aanvraag": aanvraag, "besluit": besluit})))
}

// ---------------------------------------------------------------------------
// Beoordelen (beoordelaar)
// ---------------------------------------------------------------------------

/// Proefberekening: voer de wet per onderdeel uit zonder iets vast te leggen.
pub async fn proefberekening(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if session_beoordelaar(&session).await.is_none() {
        return Err(forbidden());
    }
    let Some(aanvraag) = db::get_aanvraag(&state.pool, &id).await.map_err(internal_error)? else {
        return Err(not_found());
    };
    let uitkomst = run_engine(&state, &aanvraag).await?;
    Ok(Json(serde_json::to_value(uitkomst).map_err(internal_error)?))
}

/// Besluit vaststellen: de wet bepaalt per onderdeel; samen één beschikking.
/// Bij toekenning ontstaat één betaalopdracht aan de rechtspersoon (art. 27).
pub async fn stel_besluit_vast(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let Some(beoordelaar) = session_beoordelaar(&session).await else {
        return Err(forbidden());
    };
    let Some(aanvraag) = db::get_aanvraag(&state.pool, &id).await.map_err(internal_error)? else {
        return Err(not_found());
    };
    if aanvraag.status != "BEHANDELING" {
        return Err(bad_request(
            "Deze aanvraag is al beoordeeld of nog niet in behandeling.",
        ));
    }

    let uitkomst = run_engine(&state, &aanvraag).await?;
    let besluit_id = Uuid::new_v4().to_string();
    let vandaag = Utc::now().format("%Y-%m-%d").to_string();

    db::insert_besluit(
        &state.pool,
        &besluit_id,
        &id,
        uitkomst.subsidie_toegekend,
        uitkomst.subsidiebedrag,
        &serde_json::to_string(&uitkomst.componenten).map_err(internal_error)?,
        &uitkomst.motivering,
        &vandaag,
        &beoordelaar,
    )
    .await
    .map_err(internal_error)?;
    db::set_aanvraag_status(&state.pool, &id, "BESLUIT")
        .await
        .map_err(internal_error)?;

    // Side-effect uit art. 16/27: één betaalopdracht aan de rechtspersoon.
    if uitkomst.betaalopdracht_vereist {
        let opdracht_id = Uuid::new_v4().to_string();
        db::insert_betaalopdracht(
            &state.pool,
            &opdracht_id,
            &besluit_id,
            &aanvraag.partij_naam,
            uitkomst.betaalopdracht_bedrag,
        )
        .await
        .map_err(internal_error)?;
        tracing::info!(
            besluit = %besluit_id,
            bedrag = uitkomst.betaalopdracht_bedrag,
            "betaalopdracht aangemaakt (mock betaalsysteem)"
        );
    }

    Ok(Json(json!({
        "besluit_id": besluit_id,
        "uitkomst": uitkomst,
    })))
}

async fn run_engine(
    state: &AppState,
    aanvraag: &db::Aanvraag,
) -> Result<engine::JaaruitkomstUitkomst, ApiError> {
    let serde_json::Value::Object(eigen) = aanvraag.parameters.clone() else {
        return Err(internal_error("aanvraagparameters zijn geen object"));
    };
    let vandaag = Utc::now().format("%Y-%m-%d").to_string();
    engine::evaluate_jaaraanvraag(
        state.corpus.clone(),
        aanvraag.componenten.clone(),
        eigen,
        vandaag,
    )
    .await
    .map_err(internal_error)
}

/// Bekendmaking: het besluit wordt bekendgemaakt; de AWB (6:8) bepaalt de
/// bezwaartermijn vanaf de dag na bekendmaking.
pub async fn bekendmaking(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if session_beoordelaar(&session).await.is_none() {
        return Err(forbidden());
    }
    let Some(aanvraag) = db::get_aanvraag(&state.pool, &id).await.map_err(internal_error)? else {
        return Err(not_found());
    };
    if aanvraag.status != "BESLUIT" {
        return Err(bad_request("Er is nog geen besluit om bekend te maken."));
    }
    let Some(besluit) = db::get_besluit_by_aanvraag(&state.pool, &id)
        .await
        .map_err(internal_error)?
    else {
        return Err(not_found());
    };

    let vandaag = Utc::now().format("%Y-%m-%d").to_string();
    let termijn = engine::evaluate_bezwaartermijn(state.corpus.clone(), vandaag.clone())
        .await
        .map_err(internal_error)?;

    db::set_bekendmaking(
        &state.pool,
        &besluit.id,
        &vandaag,
        &termijn.startdatum,
        &termijn.einddatum,
    )
    .await
    .map_err(internal_error)?;
    db::set_aanvraag_status(&state.pool, &id, "BEZWAAR")
        .await
        .map_err(internal_error)?;

    Ok(Json(json!({
        "bekendmaking_datum": vandaag,
        "bezwaartermijn_startdatum": termijn.startdatum,
        "bezwaartermijn_einddatum": termijn.einddatum,
    })))
}

// ---------------------------------------------------------------------------
// Betaalopdrachten (beoordelaar)
// ---------------------------------------------------------------------------

pub async fn list_betaalopdrachten(
    State(state): State<AppState>,
    session: Session,
) -> Result<Json<serde_json::Value>, ApiError> {
    if session_beoordelaar(&session).await.is_none() {
        return Err(forbidden());
    }
    let opdrachten = db::list_betaalopdrachten(&state.pool)
        .await
        .map_err(internal_error)?;
    Ok(Json(json!(opdrachten)))
}

// ---------------------------------------------------------------------------
// Openbaar register (geen login)
// ---------------------------------------------------------------------------

pub async fn register(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let entries = db::list_register(&state.pool).await.map_err(internal_error)?;
    Ok(Json(json!(entries)))
}

pub async fn statistieken(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let stats = db::statistieken(&state.pool).await.map_err(internal_error)?;
    Ok(Json(serde_json::to_value(stats).map_err(internal_error)?))
}

#[derive(Serialize)]
pub struct Health {
    pub status: &'static str,
}

pub async fn health() -> Json<Health> {
    Json(Health { status: "ok" })
}
