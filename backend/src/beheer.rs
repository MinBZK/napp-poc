//! Beheer van het partijregister — de registratietaak van de Napp.
//!
//! Alle endpoints zijn strikt beoordelaar-only: zonder beoordelaarsessie
//! volgt 403, hetzelfde autorisatiepatroon als `handlers::list_aanvragen`.
//! Registreren van een partij, wijzigen van naam/organisatiemodel/
//! moederpartij en het corrigeren van decentrale uitslagen (toevoegen en
//! verwijderen van uitslag-regels) lopen via deze module.

use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use tower_sessions::Session;

use crate::handlers::{bad_request, forbidden, internal_error, not_found, session_beoordelaar, ApiError};
use crate::register;
use crate::state::AppState;

const ORGANEN: &[&str] = &["GEMEENTERAAD", "PROVINCIALE_STATEN", "WATERSCHAP"];

fn validate_kvk(kvk: &str) -> Result<String, ApiError> {
    let kvk = kvk.trim().to_string();
    if !kvk.chars().all(|c| c.is_ascii_digit()) || kvk.len() != 8 {
        return Err(bad_request("Vul een geldig KvK-nummer in (8 cijfers)."));
    }
    Ok(kvk)
}

fn validate_organisatiemodel(model: &str) -> Result<String, ApiError> {
    let model = model.trim().to_uppercase();
    if model != "CENTRAAL" && model != "DECENTRAAL" {
        return Err(bad_request(
            "Organisatiemodel moet CENTRAAL of DECENTRAAL zijn.",
        ));
    }
    Ok(model)
}

fn validate_naam(naam: &str) -> Result<String, ApiError> {
    let naam = naam.trim().to_string();
    if naam.is_empty() {
        return Err(bad_request("Naam is verplicht."));
    }
    Ok(naam)
}

/// Normalizes and validates an optional moederpartij reference: it must be
/// an existing registration and may not point at the party itself.
async fn validate_moederpartij(
    state: &AppState,
    eigen_kvk: &str,
    moederpartij_kvk: Option<&str>,
) -> Result<Option<String>, ApiError> {
    let Some(moeder) = moederpartij_kvk.map(str::trim).filter(|m| !m.is_empty()) else {
        return Ok(None);
    };
    if moeder == eigen_kvk {
        return Err(bad_request("Een partij kan niet haar eigen moederpartij zijn."));
    }
    if register::partij_by_kvk(&state.pool, moeder)
        .await
        .map_err(internal_error)?
        .is_none()
    {
        return Err(bad_request(&format!(
            "Moederpartij met KvK-nummer {moeder} staat niet in het register."
        )));
    }
    Ok(Some(moeder.to_string()))
}

// ---------------------------------------------------------------------------
// Partijen
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Deserialize)]
pub struct ZoekParams {
    #[serde(default)]
    pub zoek: Option<String>,
    #[serde(default)]
    pub offset: Option<i64>,
    #[serde(default)]
    pub limit: Option<i64>,
}

/// GET /api/beheer/partijen?zoek=&offset=&limit=
pub async fn list_partijen(
    State(state): State<AppState>,
    session: Session,
    Query(params): Query<ZoekParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if session_beoordelaar(&session).await.is_none() {
        return Err(forbidden());
    }
    let zoek = params.zoek.unwrap_or_default();
    let offset = params.offset.unwrap_or(0).max(0);
    let limit = params.limit.unwrap_or(25).clamp(1, 100);
    let (totaal, partijen) = register::zoek_partijen(&state.pool, &zoek, offset, limit)
        .await
        .map_err(internal_error)?;
    Ok(Json(json!({
        "totaal": totaal,
        "offset": offset,
        "limit": limit,
        "partijen": partijen,
    })))
}

async fn partij_detail(
    state: &AppState,
    kvk: &str,
) -> Result<Option<serde_json::Value>, ApiError> {
    let Some(partij) = register::partij_by_kvk(&state.pool, kvk)
        .await
        .map_err(internal_error)?
    else {
        return Ok(None);
    };
    let uitslagen = register::uitslagen_met_gebied(&state.pool, kvk)
        .await
        .map_err(internal_error)?;
    let moederpartij_naam = match &partij.moederpartij_kvk {
        Some(moeder) => register::partij_by_kvk(&state.pool, moeder)
            .await
            .map_err(internal_error)?
            .map(|p| p.naam),
        None => None,
    };
    Ok(Some(json!({
        "kvk_nummer": partij.kvk_nummer,
        "naam": partij.naam,
        "organisatiemodel": partij.organisatiemodel,
        "kamerzetels": partij.kamerzetels,
        "moederpartij_kvk": partij.moederpartij_kvk,
        "moederpartij_naam": moederpartij_naam,
        "uitslagen": uitslagen,
    })))
}

/// GET /api/beheer/partijen/{kvk} — detail incl. uitslagen met gebied-namen
/// en inwoneraantallen.
pub async fn get_partij(
    State(state): State<AppState>,
    session: Session,
    Path(kvk): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if session_beoordelaar(&session).await.is_none() {
        return Err(forbidden());
    }
    match partij_detail(&state, &kvk).await? {
        Some(detail) => Ok(Json(detail)),
        None => Err(not_found()),
    }
}

#[derive(Debug, Deserialize)]
pub struct NieuwePartij {
    pub kvk_nummer: String,
    pub naam: String,
    pub organisatiemodel: String,
    #[serde(default)]
    pub moederpartij_kvk: Option<String>,
}

/// POST /api/beheer/partijen — nieuwe registratie.
pub async fn create_partij(
    State(state): State<AppState>,
    session: Session,
    Json(body): Json<NieuwePartij>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if session_beoordelaar(&session).await.is_none() {
        return Err(forbidden());
    }
    let kvk = validate_kvk(&body.kvk_nummer)?;
    let naam = validate_naam(&body.naam)?;
    let model = validate_organisatiemodel(&body.organisatiemodel)?;
    if register::partij_by_kvk(&state.pool, &kvk)
        .await
        .map_err(internal_error)?
        .is_some()
    {
        return Err(bad_request(&format!(
            "Er staat al een partij met KvK-nummer {kvk} in het register."
        )));
    }
    let moeder = validate_moederpartij(&state, &kvk, body.moederpartij_kvk.as_deref()).await?;
    register::insert_partij(&state.pool, &kvk, &naam, &model, moeder.as_deref())
        .await
        .map_err(internal_error)?;
    match partij_detail(&state, &kvk).await? {
        Some(detail) => Ok(Json(detail)),
        None => Err(internal_error("zojuist geregistreerde partij niet gevonden")),
    }
}

#[derive(Debug, Deserialize)]
pub struct PartijWijziging {
    pub naam: String,
    pub organisatiemodel: String,
    #[serde(default)]
    pub moederpartij_kvk: Option<String>,
}

/// PUT /api/beheer/partijen/{kvk} — naam/organisatiemodel/moederpartij.
pub async fn update_partij(
    State(state): State<AppState>,
    session: Session,
    Path(kvk): Path<String>,
    Json(body): Json<PartijWijziging>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if session_beoordelaar(&session).await.is_none() {
        return Err(forbidden());
    }
    let naam = validate_naam(&body.naam)?;
    let model = validate_organisatiemodel(&body.organisatiemodel)?;
    let moeder = validate_moederpartij(&state, &kvk, body.moederpartij_kvk.as_deref()).await?;
    let updated = register::update_partij(&state.pool, &kvk, &naam, &model, moeder.as_deref())
        .await
        .map_err(internal_error)?;
    if !updated {
        return Err(not_found());
    }
    match partij_detail(&state, &kvk).await? {
        Some(detail) => Ok(Json(detail)),
        None => Err(not_found()),
    }
}

// ---------------------------------------------------------------------------
// Uitslagen
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct NieuweUitslag {
    pub orgaan: String,
    pub gebied_code: String,
    pub zetels: i64,
}

/// POST /api/beheer/partijen/{kvk}/uitslagen — uitslag-regel toevoegen.
pub async fn add_uitslag(
    State(state): State<AppState>,
    session: Session,
    Path(kvk): Path<String>,
    Json(body): Json<NieuweUitslag>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if session_beoordelaar(&session).await.is_none() {
        return Err(forbidden());
    }
    if register::partij_by_kvk(&state.pool, &kvk)
        .await
        .map_err(internal_error)?
        .is_none()
    {
        return Err(not_found());
    }
    let orgaan = body.orgaan.trim().to_uppercase();
    if !ORGANEN.contains(&orgaan.as_str()) {
        return Err(bad_request(
            "Orgaan moet GEMEENTERAAD, PROVINCIALE_STATEN of WATERSCHAP zijn.",
        ));
    }
    if body.zetels < 1 {
        return Err(bad_request("Het aantal zetels moet ten minste 1 zijn."));
    }
    let gebied_code = body.gebied_code.trim().to_string();
    if register::gebied_by_orgaan_code(&state.pool, &orgaan, &gebied_code)
        .await
        .map_err(internal_error)?
        .is_none()
    {
        return Err(bad_request(&format!(
            "Gebied {gebied_code} bestaat niet voor orgaan {orgaan}."
        )));
    }
    if register::uitslag_exists(&state.pool, &kvk, &orgaan, &gebied_code)
        .await
        .map_err(internal_error)?
    {
        return Err(bad_request(
            "Er is voor dit orgaan en gebied al een uitslag geregistreerd. Verwijder die eerst om te corrigeren.",
        ));
    }
    register::insert_uitslag(&state.pool, &kvk, &orgaan, &gebied_code, body.zetels)
        .await
        .map_err(internal_error)?;
    match partij_detail(&state, &kvk).await? {
        Some(detail) => Ok(Json(detail)),
        None => Err(not_found()),
    }
}

/// DELETE /api/beheer/partijen/{kvk}/uitslagen/{orgaan}/{gebied_code}.
pub async fn delete_uitslag(
    State(state): State<AppState>,
    session: Session,
    Path((kvk, orgaan, gebied_code)): Path<(String, String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if session_beoordelaar(&session).await.is_none() {
        return Err(forbidden());
    }
    let deleted = register::delete_uitslag(&state.pool, &kvk, &orgaan, &gebied_code)
        .await
        .map_err(internal_error)?;
    if !deleted {
        return Err(not_found());
    }
    match partij_detail(&state, &kvk).await? {
        Some(detail) => Ok(Json(detail)),
        None => Err(not_found()),
    }
}

// ---------------------------------------------------------------------------
// Gebieden (hulpdata voor het uitslag-formulier)
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Deserialize)]
pub struct GebiedParams {
    #[serde(default)]
    pub orgaan: Option<String>,
}

/// GET /api/beheer/gebieden?orgaan= — gebieden voor het uitslag-formulier.
pub async fn list_gebieden(
    State(state): State<AppState>,
    session: Session,
    Query(params): Query<GebiedParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if session_beoordelaar(&session).await.is_none() {
        return Err(forbidden());
    }
    let gebieden = register::list_gebieden(&state.pool, params.orgaan.as_deref())
        .await
        .map_err(internal_error)?;
    Ok(Json(json!(gebieden)))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::LawCorpus;
    use crate::{db, register};
    use axum::http::StatusCode;
    use regelrecht_auth::{SESSION_KEY_AUTHENTICATED, SESSION_KEY_NAME};
    use sqlx::sqlite::SqlitePoolOptions;
    use std::sync::Arc;
    use tower_sessions::Session;
    use tower_sessions_memory_store::MemoryStore;

    async fn test_state() -> AppState {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("in-memory database");
        db::init(&pool).await.expect("schema");
        // Minimal register fixture instead of the full JSON seed.
        sqlx::query(
            "INSERT INTO register_gebieden (orgaan, code, naam, inwoneraantal)
             VALUES ('GEMEENTERAAD', 'GM0344', 'Utrecht', 367984)",
        )
        .execute(&pool)
        .await
        .expect("gebied fixture");
        register::insert_partij(&pool, "11111111", "Testpartij", "CENTRAAL", None)
            .await
            .expect("partij fixture");
        AppState {
            pool,
            corpus: Arc::new(LawCorpus {
                wpp: String::new(),
                regeling: String::new(),
                besluit_decentraal: String::new(),
                awb: String::new(),
                termijnenwet: String::new(),
            }),
            oidc_client: None,
            oidc_config: None,
            end_session_url: None,
            base_url: None,
            http_client: reqwest::Client::new(),
        }
    }

    fn anonymous_session() -> Session {
        Session::new(None, Arc::new(MemoryStore::default()), None)
    }

    async fn beoordelaar_session() -> Session {
        let session = anonymous_session();
        session
            .insert(SESSION_KEY_AUTHENTICATED, true)
            .await
            .expect("session insert");
        session
            .insert(SESSION_KEY_NAME, "Testbeoordelaar".to_string())
            .await
            .expect("session insert");
        session
    }

    fn fout(err: &ApiError) -> String {
        err.1["fout"].as_str().unwrap_or_default().to_string()
    }

    #[tokio::test]
    async fn beheer_vereist_beoordelaarsessie() {
        let state = test_state().await;
        let err = list_partijen(
            State(state.clone()),
            anonymous_session(),
            Query(ZoekParams::default()),
        )
        .await
        .expect_err("zonder sessie hoort 403");
        assert_eq!(err.0, StatusCode::FORBIDDEN);

        let err = create_partij(
            State(state),
            anonymous_session(),
            Json(NieuwePartij {
                kvk_nummer: "22222222".into(),
                naam: "Partij".into(),
                organisatiemodel: "CENTRAAL".into(),
                moederpartij_kvk: None,
            }),
        )
        .await
        .expect_err("zonder sessie hoort 403");
        assert_eq!(err.0, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn create_partij_valideert_invoer() {
        let state = test_state().await;

        // Ongeldig KvK-nummer.
        let err = create_partij(
            State(state.clone()),
            beoordelaar_session().await,
            Json(NieuwePartij {
                kvk_nummer: "123".into(),
                naam: "Partij".into(),
                organisatiemodel: "CENTRAAL".into(),
                moederpartij_kvk: None,
            }),
        )
        .await
        .expect_err("kvk moet 8 cijfers zijn");
        assert_eq!(err.0, StatusCode::BAD_REQUEST);
        assert!(fout(&err).contains("KvK"));

        // Bestaand KvK-nummer.
        let err = create_partij(
            State(state.clone()),
            beoordelaar_session().await,
            Json(NieuwePartij {
                kvk_nummer: "11111111".into(),
                naam: "Dubbel".into(),
                organisatiemodel: "CENTRAAL".into(),
                moederpartij_kvk: None,
            }),
        )
        .await
        .expect_err("kvk moet uniek zijn");
        assert!(fout(&err).contains("al een partij"));

        // Onbekend organisatiemodel.
        let err = create_partij(
            State(state.clone()),
            beoordelaar_session().await,
            Json(NieuwePartij {
                kvk_nummer: "33333333".into(),
                naam: "Partij".into(),
                organisatiemodel: "FEDERAAL".into(),
                moederpartij_kvk: None,
            }),
        )
        .await
        .expect_err("organisatiemodel beperkt tot CENTRAAL|DECENTRAAL");
        assert!(fout(&err).contains("Organisatiemodel"));

        // Niet-bestaande moederpartij.
        let err = create_partij(
            State(state.clone()),
            beoordelaar_session().await,
            Json(NieuwePartij {
                kvk_nummer: "33333333".into(),
                naam: "Afdeling".into(),
                organisatiemodel: "DECENTRAAL".into(),
                moederpartij_kvk: Some("99999999".into()),
            }),
        )
        .await
        .expect_err("moederpartij moet bestaan");
        assert!(fout(&err).contains("Moederpartij"));

        // Geldige registratie slaagt en is daarna vindbaar.
        let detail = create_partij(
            State(state.clone()),
            beoordelaar_session().await,
            Json(NieuwePartij {
                kvk_nummer: "33333333".into(),
                naam: "Afdeling Utrecht".into(),
                organisatiemodel: "DECENTRAAL".into(),
                moederpartij_kvk: Some("11111111".into()),
            }),
        )
        .await
        .expect("geldige registratie");
        assert_eq!(detail.0["naam"], "Afdeling Utrecht");
        assert_eq!(detail.0["moederpartij_naam"], "Testpartij");

        let lijst = list_partijen(
            State(state),
            beoordelaar_session().await,
            Query(ZoekParams {
                zoek: Some("Afdeling".into()),
                ..ZoekParams::default()
            }),
        )
        .await
        .expect("zoeken");
        assert_eq!(lijst.0["totaal"], 1);
        assert_eq!(lijst.0["partijen"][0]["kvk_nummer"], "33333333");
    }

    #[tokio::test]
    async fn uitslagen_valideren_en_muteren() {
        let state = test_state().await;

        // Onbekend gebied.
        let err = add_uitslag(
            State(state.clone()),
            beoordelaar_session().await,
            Path("11111111".into()),
            Json(NieuweUitslag {
                orgaan: "GEMEENTERAAD".into(),
                gebied_code: "GM9999".into(),
                zetels: 3,
            }),
        )
        .await
        .expect_err("gebied moet bestaan");
        assert!(fout(&err).contains("bestaat niet"));

        // Zetels < 1.
        let err = add_uitslag(
            State(state.clone()),
            beoordelaar_session().await,
            Path("11111111".into()),
            Json(NieuweUitslag {
                orgaan: "GEMEENTERAAD".into(),
                gebied_code: "GM0344".into(),
                zetels: 0,
            }),
        )
        .await
        .expect_err("zetels moeten >= 1 zijn");
        assert!(fout(&err).contains("zetels"));

        // Toevoegen slaagt, dubbel toevoegen niet.
        let detail = add_uitslag(
            State(state.clone()),
            beoordelaar_session().await,
            Path("11111111".into()),
            Json(NieuweUitslag {
                orgaan: "GEMEENTERAAD".into(),
                gebied_code: "GM0344".into(),
                zetels: 3,
            }),
        )
        .await
        .expect("geldige uitslag");
        assert_eq!(detail.0["uitslagen"][0]["gebied_naam"], "Utrecht");
        assert_eq!(detail.0["uitslagen"][0]["inwoneraantal"], 367984);

        let err = add_uitslag(
            State(state.clone()),
            beoordelaar_session().await,
            Path("11111111".into()),
            Json(NieuweUitslag {
                orgaan: "GEMEENTERAAD".into(),
                gebied_code: "GM0344".into(),
                zetels: 5,
            }),
        )
        .await
        .expect_err("dubbele uitslag hoort te falen");
        assert!(fout(&err).contains("al een uitslag"));

        // Verwijderen, daarna nogmaals verwijderen → 404.
        let detail = delete_uitslag(
            State(state.clone()),
            beoordelaar_session().await,
            Path(("11111111".into(), "GEMEENTERAAD".into(), "GM0344".into())),
        )
        .await
        .expect("verwijderen");
        assert!(detail.0["uitslagen"].as_array().expect("array").is_empty());

        let err = delete_uitslag(
            State(state),
            beoordelaar_session().await,
            Path(("11111111".into(), "GEMEENTERAAD".into(), "GM0344".into())),
        )
        .await
        .expect_err("tweede verwijdering hoort 404 te geven");
        assert_eq!(err.0, StatusCode::NOT_FOUND);
    }
}
