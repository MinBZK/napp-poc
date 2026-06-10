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
use crate::machtiging;
use crate::state::AppState;

const SESSION_KEY_EH_KVK: &str = "eh_kvk";
const SESSION_KEY_EH_PARTIJ: &str = "eh_partij";

pub(crate) type ApiError = (StatusCode, Json<serde_json::Value>);

pub(crate) fn internal_error(e: impl std::fmt::Display) -> ApiError {
    tracing::error!(error = %e, "interne fout");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({"fout": "Er is een interne fout opgetreden."})),
    )
}

pub(crate) fn bad_request(msg: &str) -> ApiError {
    (StatusCode::BAD_REQUEST, Json(json!({"fout": msg})))
}

pub(crate) fn not_found() -> ApiError {
    (
        StatusCode::NOT_FOUND,
        Json(json!({"fout": "Niet gevonden."})),
    )
}

pub(crate) fn forbidden() -> ApiError {
    forbidden_with("Geen toegang.")
}

fn forbidden_with(msg: &str) -> ApiError {
    (StatusCode::FORBIDDEN, Json(json!({"fout": msg})))
}

async fn session_kvk(session: &Session) -> Option<String> {
    session.get(SESSION_KEY_EH_KVK).await.ok().flatten()
}

/// Het subsidiejaar waarop een aanvraag van vandaag betrekking heeft.
/// De subsidie wordt per kalenderjaar verstrekt en moet uiterlijk
/// 1 november voorafgaand aan het subsidiejaar worden aangevraagd
/// (Wpp art. 17). De orchestratie routeert een aanvraag daarom naar het
/// eerstvolgende subsidiejaar waarvoor de termijn nog niet is verstreken:
/// tot en met 1 november is dat het komende jaar, daarna het jaar erop.
fn subsidiejaar_voor(datum: &str) -> i64 {
    let parsed = chrono::NaiveDate::parse_from_str(datum, "%Y-%m-%d")
        .unwrap_or_else(|_| Utc::now().date_naive());
    let jaar = chrono::Datelike::year(&parsed) as i64;
    let deadline = chrono::NaiveDate::from_ymd_opt(chrono::Datelike::year(&parsed), 11, 1)
        .expect("1 november bestaat");
    if parsed <= deadline {
        jaar + 1
    } else {
        jaar + 2
    }
}

pub(crate) async fn session_beoordelaar(session: &Session) -> Option<String> {
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
    /// Optional representation profile; absent means VOLLEDIG (backwards
    /// compatible with existing clients and the seed script).
    #[serde(default)]
    pub machtiging: Option<machtiging::Machtiging>,
}

/// MOCK eHerkenning-login voor aanvragers. eHerkenning levert alleen de
/// identiteit van de rechtspersoon (KvK); de partijnaam volgt uit het
/// partijregister van de Napp. Een onbekend KvK-nummer kan gewoon inloggen
/// en aanvragen — de wet wijst dan af (AWB 4:1: iedereen mag aanvragen).
/// De optionele machtiging beperkt de sessie tot één afdeling (volmacht).
pub async fn eherkenning_login(
    State(state): State<AppState>,
    session: Session,
    Json(body): Json<EherkenningLogin>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let kvk = body.kvk_nummer.trim().to_string();
    if !kvk.chars().all(|c| c.is_ascii_digit()) || kvk.len() != 8 {
        return Err(bad_request("Vul een geldig KVK-nummer in (8 cijfers)."));
    }
    let m = body.machtiging.unwrap_or_default();
    machtiging::valideer(&state.pool, &kvk, &m)
        .await
        .map_err(|e| match e {
            machtiging::ValidatieFout::Ongeldig(msg) => bad_request(&msg),
            machtiging::ValidatieFout::Intern(e) => internal_error(e),
        })?;
    let naam = match crate::register::partij_by_kvk(&state.pool, &kvk)
        .await
        .map_err(internal_error)?
    {
        Some(partij) => partij.naam,
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
    session
        .insert(machtiging::SESSION_KEY_EH_MACHTIGING, m.clone())
        .await
        .map_err(internal_error)?;
    let machtiging_json = m.to_json(&state.pool).await.map_err(internal_error)?;
    Ok(Json(json!({
        "rol": "aanvrager",
        "kvk_nummer": kvk,
        "partij_naam": naam,
        "machtiging": machtiging_json,
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
    let vandaag = Utc::now().format("%Y-%m-%d").to_string();
    let jaar = subsidiejaar_voor(&vandaag);
    let partij = crate::register::partij_by_kvk(&state.pool, &kvk)
        .await
        .map_err(internal_error)?;
    let bezet = db::bezette_componenten(&state.pool, &kvk, jaar)
        .await
        .map_err(internal_error)?;
    let termijnen = engine::evaluate_termijnen(state.corpus.clone(), jaar, vandaag)
        .await
        .map_err(internal_error)?;

    // A limited machtiging (branch volmacht) only sees its own area.
    let m = machtiging::session_machtiging(&session).await;
    let componenten = m.filter_componenten(
        aanspraken_voor(&state.pool, &kvk)
            .await
            .map_err(internal_error)?,
    );
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
        "aanvraagtermijn_einddatum": termijnen.aanvraagtermijn_einddatum,
        "beslistermijn_einddatum": termijnen.beslistermijn_einddatum,
        "aanspraken": aanspraken,
    })))
}

/// Bouw de componenten (aanspraken) van een rechtspersoon uit het register.
/// Een onbekende organisatie krijgt één landelijke component met nul zetels:
/// de aanvraagroute blijft open (AWB 4:1), de wet wijst af.
async fn aanspraken_voor(
    pool: &sqlx::SqlitePool,
    kvk: &str,
) -> anyhow::Result<Vec<db::Component>> {
    let Some(partij) = crate::register::partij_by_kvk(pool, kvk).await? else {
        return Ok(vec![db::Component {
            key: "LANDELIJK".into(),
            soort: "LANDELIJK".into(),
            orgaan: None,
            gebied_code: None,
            gebied: None,
            zetels: 0,
            inwoneraantal: 0,
        }]);
    };
    let uitslagen = crate::register::uitslagen_met_gebied(pool, kvk).await?;
    let mut componenten = Vec::new();
    // Landelijke component alleen voor partijen met kamerzetels; een
    // afdeling of lokale partij heeft die aanspraak niet.
    if partij.kamerzetels > 0 || uitslagen.is_empty() {
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
    for u in uitslagen {
        componenten.push(db::Component {
            key: format!("{}:{}", u.orgaan, u.gebied_code),
            soort: "DECENTRAAL".into(),
            orgaan: Some(u.orgaan),
            gebied_code: Some(u.gebied_code),
            gebied: u.gebied_naam,
            zetels: u.zetels,
            inwoneraantal: u.inwoneraantal,
        });
    }
    Ok(componenten)
}

/// Eigen opgaven voor de landelijke component: ledental en de aangewezen
/// neveninstellingen (Wpp art. 3, 4 en 14, onderdelen b-d).
fn neem_landelijke_opgaven_over(
    bron: &serde_json::Map<String, serde_json::Value>,
    eigen: &mut serde_json::Map<String, serde_json::Value>,
) {
    let leden = bron
        .get("aantal_betalende_leden")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    eigen.insert("aantal_betalende_leden".to_string(), json!(leden));
    for key in [
        "heeft_wetenschappelijk_instituut",
        "heeft_jongerenorganisatie",
        "heeft_instelling_buitenland",
    ] {
        let waarde = bron.get(key).and_then(|v| v.as_bool()).unwrap_or(false);
        eigen.insert(key.to_string(), json!(waarde));
    }
    let pjo_leden = bron
        .get("aantal_leden_jongerenorganisatie")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    eigen.insert(
        "aantal_leden_jongerenorganisatie".to_string(),
        json!(pjo_leden),
    );
}

/// De noemer van de ledencomponent voor een (proef)berekening: de opgaven
/// die al in de aanvragentabel staan, plus — als de eigen aanvraag daar nog
/// niet bij zit — de eigen opgave voor zover die aan de ledeneis voldoet.
async fn ledentotaal_voor(
    state: &AppState,
    jaar: i64,
    eigen_leden: Option<i64>,
) -> Result<i64, ApiError> {
    let mut totaal = db::totaal_opgegeven_leden(&state.pool, jaar)
        .await
        .map_err(internal_error)?;
    if let Some(leden) = eigen_leden {
        if leden >= 1000 {
            totaal += leden;
        }
    }
    Ok(totaal)
}

/// Demo-voorbeelden voor de gemockte eHerkenning-login (alleen metadata).
pub async fn register_demo() -> Json<serde_json::Value> {
    Json(json!(crate::register::demo_voorbeelden()))
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
    session
        .remove::<machtiging::Machtiging>(machtiging::SESSION_KEY_EH_MACHTIGING)
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
pub async fn me(
    State(state): State<AppState>,
    session: Session,
) -> Result<Json<serde_json::Value>, ApiError> {
    let kvk = session_kvk(&session).await;
    let partij: Option<String> = session.get(SESSION_KEY_EH_PARTIJ).await.ok().flatten();
    let m = machtiging::session_machtiging(&session).await;
    let machtiging_json = m.to_json(&state.pool).await.map_err(internal_error)?;
    let beoordelaar = session_beoordelaar(&session).await;
    Ok(Json(json!({
        "aanvrager": kvk.map(|k| json!({
            "kvk_nummer": k,
            "partij_naam": partij,
            "machtiging": machtiging_json,
        })),
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
    let vandaag = Utc::now().format("%Y-%m-%d").to_string();
    let jaar = subsidiejaar_voor(&vandaag);

    if body.componenten.is_empty() {
        return Err(bad_request("Kies ten minste één onderdeel om aan te vragen."));
    }

    // De componenten komen uit het register, nooit uit de client: de client
    // stuurt alleen sleutels; gegevens (zetels, inwoneraantallen) worden hier
    // bevroren op registerwaarden.
    let m = machtiging::session_machtiging(&session).await;
    let beschikbaar = aanspraken_voor(&state.pool, &kvk)
        .await
        .map_err(internal_error)?;
    let mut componenten: Vec<db::Component> = Vec::new();
    for key in &body.componenten {
        if !m.allows_key(key) {
            return Err(forbidden_with(&format!(
                "Uw machtiging als afdelingsbestuurder geldt niet voor onderdeel '{key}'. \
                 Log in namens de gehele partij om dit onderdeel aan te vragen."
            )));
        }
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

    // Eigen opgaven: verklaringen verplicht; ledental en neveninstellingen
    // optioneel (alleen relevant voor de landelijke component).
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
    neem_landelijke_opgaven_over(&body.parameters, &mut eigen);

    let id = Uuid::new_v4().to_string();
    // Wpp art. 17 (lex specialis t.o.v. AWB 4:13): de Napp besluit voor
    // 1 januari van het subsidiejaar.
    let termijnen = engine::evaluate_termijnen(state.corpus.clone(), jaar, vandaag.clone())
        .await
        .map_err(internal_error)?;
    let beslistermijn = termijnen.beslistermijn_einddatum;
    db::insert_aanvraag(
        &state.pool,
        &id,
        &kvk,
        &partij,
        jaar,
        &serde_json::to_string(&componenten).map_err(internal_error)?,
        &serde_json::to_string(&eigen).map_err(internal_error)?,
        &vandaag,
        Some(&beslistermijn),
    )
    .await
    .map_err(internal_error)?;

    Ok(Json(json!({
        "id": id,
        "status": "BEHANDELING",
        "subsidiejaar": jaar,
        "beslistermijn_einddatum": beslistermijn,
    })))
}

/// Indicatieve berekening voor de aanvrager: de wet uitgevoerd op de
/// gekozen onderdelen, zonder iets vast te leggen. Zo ziet de partij bij het
/// invullen al wat de wet zou beslissen.
pub async fn proef_aanspraken(
    State(state): State<AppState>,
    session: Session,
    Json(body): Json<NieuweAanvraag>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let Some(kvk) = session_kvk(&session).await else {
        return Err(forbidden());
    };
    let m = machtiging::session_machtiging(&session).await;
    let beschikbaar = m.filter_componenten(
        aanspraken_voor(&state.pool, &kvk)
            .await
            .map_err(internal_error)?,
    );
    let componenten: Vec<db::Component> = beschikbaar
        .into_iter()
        .filter(|c| body.componenten.contains(&c.key))
        .collect();
    if componenten.is_empty() {
        return Err(bad_request("Geen onderdelen gekozen."));
    }
    let mut eigen = serde_json::Map::new();
    for key in [
        "ontvangt_anonieme_giften",
        "ontvangt_giften_niet_ingezetenen",
        "voldoet_aan_meldplicht_giften",
        "financien_openbaar_op_website",
    ] {
        let waarde = body.parameters.get(key).and_then(|v| v.as_bool()).unwrap_or(false);
        eigen.insert(key.to_string(), json!(waarde));
    }
    neem_landelijke_opgaven_over(&body.parameters, &mut eigen);

    let vandaag = Utc::now().format("%Y-%m-%d").to_string();
    let jaar = subsidiejaar_voor(&vandaag);
    // De eigen opgave telt mee in de noemer van de ledencomponent als de
    // landelijke component onderdeel is van deze (proef)aanvraag.
    let eigen_leden = componenten
        .iter()
        .any(|c| c.soort == "LANDELIJK")
        .then(|| eigen.get("aantal_betalende_leden").and_then(|v| v.as_i64()).unwrap_or(0));
    let totaal_leden = ledentotaal_voor(&state, jaar, eigen_leden).await?;
    let uitkomst = engine::evaluate_jaaraanvraag(
        state.corpus.clone(),
        componenten,
        eigen,
        vandaag,
        jaar,
        totaal_leden,
    )
    .await
    .map_err(internal_error)?;
    Ok(Json(json!({
        "subsidie_toegekend": uitkomst.subsidie_toegekend,
        "subsidiebedrag": uitkomst.subsidiebedrag,
        "voorschot_bedrag": uitkomst.betaalopdracht_bedrag,
        "subsidiejaar": jaar,
        "onderdelen_toegekend": uitkomst.componenten.iter().filter(|c| c.toegekend).count(),
        "onderdelen_totaal": uitkomst.componenten.len(),
    })))
}

async fn aanvragen_met_besluit(
    state: &AppState,
    aanvragen: Vec<db::Aanvraag>,
) -> Result<Vec<serde_json::Value>, ApiError> {
    let mut result = Vec::with_capacity(aanvragen.len());
    for aanvraag in aanvragen {
        let besluit = db::get_besluit_by_aanvraag(&state.pool, &aanvraag.id)
            .await
            .map_err(internal_error)?;
        result.push(json!({"aanvraag": aanvraag, "besluit": besluit}));
    }
    Ok(result)
}

/// Aanvragersportaal: uitsluitend de aanvragen van de eigen rechtspersoon.
/// Bewust een eigen endpoint: een sessie kan beide rollen tegelijk dragen
/// en mag in dit portaal nooit andermans dossiers zien.
pub async fn list_mijn_aanvragen(
    State(state): State<AppState>,
    session: Session,
) -> Result<Json<serde_json::Value>, ApiError> {
    let Some(kvk) = session_kvk(&session).await else {
        return Err(forbidden());
    };
    // A limited machtiging only sees dossiers that fall entirely within its
    // scope; other dossiers of the same legal entity stay hidden.
    let m = machtiging::session_machtiging(&session).await;
    let aanvragen: Vec<db::Aanvraag> = db::list_aanvragen(&state.pool, Some(&kvk))
        .await
        .map_err(internal_error)?
        .into_iter()
        .filter(|a| m.covers(&a.componenten))
        .collect();
    Ok(Json(json!(aanvragen_met_besluit(&state, aanvragen).await?)))
}

pub async fn get_mijn_aanvraag(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let Some(kvk) = session_kvk(&session).await else {
        return Err(forbidden());
    };
    let Some(aanvraag) = db::get_aanvraag(&state.pool, &id).await.map_err(internal_error)? else {
        return Err(not_found());
    };
    if aanvraag.kvk_nummer != kvk {
        return Err(not_found());
    }
    // Outside the machtiging = does not exist for this session (404, not 403:
    // a branch board member should not learn about other dossiers).
    let m = machtiging::session_machtiging(&session).await;
    if !m.covers(&aanvraag.componenten) {
        return Err(not_found());
    }
    let besluit = db::get_besluit_by_aanvraag(&state.pool, &id)
        .await
        .map_err(internal_error)?;
    Ok(Json(json!({"aanvraag": aanvraag, "besluit": besluit})))
}

/// Beoordelingsomgeving: de volledige werkvoorraad.
pub async fn list_aanvragen(
    State(state): State<AppState>,
    session: Session,
) -> Result<Json<serde_json::Value>, ApiError> {
    if session_beoordelaar(&session).await.is_none() {
        return Err(forbidden());
    }
    let aanvragen = db::list_aanvragen(&state.pool, None)
        .await
        .map_err(internal_error)?;
    Ok(Json(json!(aanvragen_met_besluit(&state, aanvragen).await?)))
}

pub async fn get_aanvraag(
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
    // De eigen aanvraag staat al in de aanvragentabel en telt dus al mee in
    // het ledentotaal van het subsidiejaar.
    let totaal_leden = ledentotaal_voor(state, aanvraag.subsidiejaar, None).await?;
    engine::evaluate_jaaraanvraag(
        state.corpus.clone(),
        aanvraag.componenten.clone(),
        eigen,
        vandaag,
        aanvraag.subsidiejaar,
        totaal_leden,
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
