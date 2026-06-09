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

/// Registergegevens van de ingelogde partij: landelijke zetels (Kiesraad),
/// decentrale uitslagen (Kiesraad) en gebieden met inwoneraantal (CBS).
pub async fn mijn_registratie(session: Session) -> Result<Json<serde_json::Value>, ApiError> {
    let Some(kvk) = session_kvk(&session).await else {
        return Err(forbidden());
    };
    let partij = crate::register::partij_by_kvk(&kvk);
    let uitslagen: Vec<serde_json::Value> = partij
        .map(|p| {
            p.decentrale_uitslagen
                .iter()
                .map(|u| {
                    let gebied = crate::register::gebied_by_code(&u.gebied_code);
                    json!({
                        "orgaan": u.orgaan,
                        "gebied_code": u.gebied_code,
                        "gebied": gebied.map(|g| g.naam.clone()),
                        "zetels": u.zetels,
                        "inwoneraantal": gebied.map(|g| g.inwoneraantal).unwrap_or(0),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(Json(json!({
        "partij": partij.map(|p| json!({"naam": p.naam, "kamerzetels": p.kamerzetels})),
        "decentrale_uitslagen": uitslagen,
        "gebieden": crate::register::register().gebieden,
    })))
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

    // Zetels en inwoneraantal komen uit de officiële bronnen (Kiesraad, CBS)
    // via het partijregister — wat de client daarvoor instuurt wordt
    // genegeerd. Alleen eigen opgaven (ledenaantal, verklaringen) komen uit
    // het formulier. Onbekend in het register betekent nul zetels: de
    // aanvraag mag worden ingediend, de wet wijst af.
    let mut params = serde_json::Map::new();
    params.insert("niveau".to_string(), json!(body.niveau));

    if body.niveau == "LANDELIJK" {
        let kamerzetels = crate::register::partij_by_kvk(&kvk)
            .map(|p| p.kamerzetels)
            .unwrap_or(0);
        params.insert("aantal_kamerzetels".to_string(), json!(kamerzetels));
        let leden = body
            .parameters
            .get("aantal_betalende_leden")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        params.insert("aantal_betalende_leden".to_string(), json!(leden));
        params.insert("aantal_raadszetels".to_string(), json!(0));
        params.insert("inwoneraantal_gemeente".to_string(), json!(0));
    } else {
        let Some(gebied_code) = body.gemeente.as_deref().filter(|g| !g.trim().is_empty()) else {
            return Err(bad_request("Kies een gemeente voor een decentrale aanvraag."));
        };
        let Some(gebied) = crate::register::gebied_by_code(gebied_code) else {
            return Err(bad_request("Onbekend gebied."));
        };
        let raadszetels = crate::register::uitslag_by_kvk_gebied(&kvk, gebied_code)
            .map(|u| u.zetels)
            .unwrap_or(0);
        let inwoners = gebied.inwoneraantal;
        params.insert("aantal_raadszetels".to_string(), json!(raadszetels));
        params.insert("inwoneraantal_gemeente".to_string(), json!(inwoners));
        params.insert("aantal_kamerzetels".to_string(), json!(0));
        params.insert("aantal_betalende_leden".to_string(), json!(0));
    }

    for key in [
        "ontvangt_anonieme_giften",
        "ontvangt_giften_niet_ingezetenen",
        "voldoet_aan_meldplicht_giften",
        "financien_openbaar_op_website",
    ] {
        let Some(waarde) = body.parameters.get(key).and_then(|v| v.as_bool()) else {
            return Err(bad_request(&format!(
                "Verplichte verklaring '{key}' ontbreekt."
            )));
        };
        params.insert(key.to_string(), json!(waarde));
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
