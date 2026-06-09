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
    pub partij_naam: String,
}

/// MOCK eHerkenning-login voor aanvragers (politieke partijen).
pub async fn eherkenning_login(
    session: Session,
    Json(body): Json<EherkenningLogin>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if body.kvk_nummer.trim().is_empty() || body.partij_naam.trim().is_empty() {
        return Err(bad_request("KVK-nummer en partijnaam zijn verplicht."));
    }
    session
        .insert(SESSION_KEY_EH_KVK, body.kvk_nummer.trim().to_string())
        .await
        .map_err(internal_error)?;
    session
        .insert(SESSION_KEY_EH_PARTIJ, body.partij_naam.trim().to_string())
        .await
        .map_err(internal_error)?;
    Ok(Json(json!({
        "rol": "aanvrager",
        "kvk_nummer": body.kvk_nummer.trim(),
        "partij_naam": body.partij_naam.trim(),
        "mock": true,
    })))
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
    pub niveau: String,
    pub gemeente: Option<String>,
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
        .unwrap_or_else(|| "Onbekende partij".to_string());

    if body.niveau != "LANDELIJK" && body.niveau != "DECENTRAAL" {
        return Err(bad_request("Niveau moet LANDELIJK of DECENTRAAL zijn."));
    }

    // De engine resolvet alle inputs eager — vul ontbrekende parameters van
    // de andere track aan met neutrale waarden.
    let mut params = body.parameters.clone();
    params.insert("niveau".to_string(), json!(body.niveau));
    for (key, default) in [
        ("aantal_kamerzetels", json!(0)),
        ("aantal_betalende_leden", json!(0)),
        ("aantal_raadszetels", json!(0)),
        ("inwoneraantal_gemeente", json!(0)),
    ] {
        params.entry(key.to_string()).or_insert(default);
    }
    for key in [
        "ontvangt_anonieme_giften",
        "ontvangt_giften_niet_ingezetenen",
        "voldoet_aan_meldplicht_giften",
        "financien_openbaar_op_website",
    ] {
        if !params.contains_key(key) {
            return Err(bad_request(&format!(
                "Verplichte verklaring '{key}' ontbreekt."
            )));
        }
    }

    let id = Uuid::new_v4().to_string();
    let vandaag = Utc::now().format("%Y-%m-%d").to_string();
    let params_json = serde_json::to_string(&params).map_err(internal_error)?;
    db::insert_aanvraag(
        &state.pool,
        &id,
        &kvk,
        &partij,
        &body.niveau,
        body.gemeente.as_deref(),
        &params_json,
        &vandaag,
    )
    .await
    .map_err(internal_error)?;

    Ok(Json(json!({"id": id, "status": "BEHANDELING"})))
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

/// Proefberekening: voer de wet uit zonder iets vast te leggen.
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

/// Besluit vaststellen: de engine bepaalt de uitkomst, de beoordelaar stelt
/// het besluit vast. Bij toekenning ontstaat automatisch een betaalopdracht
/// (de side-effect uit artikel 16 van de wet).
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
    let outputs_json = serde_json::to_string(&uitkomst.outputs).map_err(internal_error)?;

    db::insert_besluit(
        &state.pool,
        &besluit_id,
        &id,
        uitkomst.subsidie_toegekend,
        uitkomst.subsidiebedrag,
        &outputs_json,
        &uitkomst.motivering,
        &vandaag,
        &beoordelaar,
    )
    .await
    .map_err(internal_error)?;
    db::set_aanvraag_status(&state.pool, &id, "BESLUIT")
        .await
        .map_err(internal_error)?;

    // Side-effect uit art. 16: betaalopdracht naar het (gemockte) betaalsysteem.
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

async fn run_engine(
    state: &AppState,
    aanvraag: &db::Aanvraag,
) -> Result<engine::BesluitUitkomst, ApiError> {
    let serde_json::Value::Object(params_map) = &aanvraag.parameters else {
        return Err(internal_error("aanvraagparameters zijn geen object"));
    };
    let params = engine::json_params_to_engine(params_map);
    let vandaag = Utc::now().format("%Y-%m-%d").to_string();
    engine::evaluate_besluit(state.corpus.clone(), params, vandaag)
        .await
        .map_err(internal_error)
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
