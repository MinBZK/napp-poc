//! SQLite persistence: aanvragen, besluiten, betaalopdrachten.
//!
//! The besluit-state follows RFC-008: the engine stays stateless per stage;
//! this layer persists the accumulated outputs and the current stage.

use serde::Serialize;
use sqlx::{Row, SqlitePool};

pub async fn init(pool: &SqlitePool) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS aanvragen (
            id TEXT PRIMARY KEY,
            kvk_nummer TEXT NOT NULL,
            partij_naam TEXT NOT NULL,
            niveau TEXT NOT NULL,
            gemeente TEXT,
            parameters TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'BEHANDELING',
            aanvraag_datum TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE TABLE IF NOT EXISTS besluiten (
            id TEXT PRIMARY KEY,
            aanvraag_id TEXT NOT NULL UNIQUE REFERENCES aanvragen(id),
            subsidie_toegekend INTEGER NOT NULL,
            subsidiebedrag INTEGER NOT NULL,
            outputs TEXT NOT NULL,
            motivering TEXT NOT NULL,
            besluit_datum TEXT NOT NULL,
            bekendmaking_datum TEXT,
            bezwaartermijn_startdatum TEXT,
            bezwaartermijn_einddatum TEXT,
            beoordelaar TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE TABLE IF NOT EXISTS betaalopdrachten (
            id TEXT PRIMARY KEY,
            besluit_id TEXT NOT NULL REFERENCES besluiten(id),
            partij_naam TEXT NOT NULL,
            bedrag INTEGER NOT NULL,
            status TEXT NOT NULL DEFAULT 'AANGEMAAKT',
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        "#,
    )
    .execute(pool)
    .await?;
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct Aanvraag {
    pub id: String,
    pub kvk_nummer: String,
    pub partij_naam: String,
    pub niveau: String,
    pub gemeente: Option<String>,
    pub parameters: serde_json::Value,
    pub status: String,
    pub aanvraag_datum: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct Besluit {
    pub id: String,
    pub aanvraag_id: String,
    pub subsidie_toegekend: bool,
    pub subsidiebedrag: i64,
    pub outputs: serde_json::Value,
    pub motivering: String,
    pub besluit_datum: String,
    pub bekendmaking_datum: Option<String>,
    pub bezwaartermijn_startdatum: Option<String>,
    pub bezwaartermijn_einddatum: Option<String>,
    pub beoordelaar: String,
}

#[derive(Debug, Serialize)]
pub struct Betaalopdracht {
    pub id: String,
    pub besluit_id: String,
    pub partij_naam: String,
    pub bedrag: i64,
    pub status: String,
    pub created_at: String,
}

fn row_to_aanvraag(row: &sqlx::sqlite::SqliteRow) -> Aanvraag {
    Aanvraag {
        id: row.get("id"),
        kvk_nummer: row.get("kvk_nummer"),
        partij_naam: row.get("partij_naam"),
        niveau: row.get("niveau"),
        gemeente: row.get("gemeente"),
        parameters: serde_json::from_str(row.get::<String, _>("parameters").as_str())
            .unwrap_or(serde_json::Value::Null),
        status: row.get("status"),
        aanvraag_datum: row.get("aanvraag_datum"),
        created_at: row.get("created_at"),
    }
}

fn row_to_besluit(row: &sqlx::sqlite::SqliteRow) -> Besluit {
    Besluit {
        id: row.get("id"),
        aanvraag_id: row.get("aanvraag_id"),
        subsidie_toegekend: row.get::<i64, _>("subsidie_toegekend") != 0,
        subsidiebedrag: row.get("subsidiebedrag"),
        outputs: serde_json::from_str(row.get::<String, _>("outputs").as_str())
            .unwrap_or(serde_json::Value::Null),
        motivering: row.get("motivering"),
        besluit_datum: row.get("besluit_datum"),
        bekendmaking_datum: row.get("bekendmaking_datum"),
        bezwaartermijn_startdatum: row.get("bezwaartermijn_startdatum"),
        bezwaartermijn_einddatum: row.get("bezwaartermijn_einddatum"),
        beoordelaar: row.get("beoordelaar"),
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn insert_aanvraag(
    pool: &SqlitePool,
    id: &str,
    kvk_nummer: &str,
    partij_naam: &str,
    niveau: &str,
    gemeente: Option<&str>,
    parameters_json: &str,
    aanvraag_datum: &str,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO aanvragen (id, kvk_nummer, partij_naam, niveau, gemeente, parameters, aanvraag_datum)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(kvk_nummer)
    .bind(partij_naam)
    .bind(niveau)
    .bind(gemeente)
    .bind(parameters_json)
    .bind(aanvraag_datum)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_aanvragen(
    pool: &SqlitePool,
    kvk_filter: Option<&str>,
) -> anyhow::Result<Vec<Aanvraag>> {
    let rows = match kvk_filter {
        Some(kvk) => {
            sqlx::query("SELECT * FROM aanvragen WHERE kvk_nummer = ? ORDER BY created_at DESC")
                .bind(kvk)
                .fetch_all(pool)
                .await?
        }
        None => {
            sqlx::query("SELECT * FROM aanvragen ORDER BY created_at DESC")
                .fetch_all(pool)
                .await?
        }
    };
    Ok(rows.iter().map(row_to_aanvraag).collect())
}

pub async fn get_aanvraag(pool: &SqlitePool, id: &str) -> anyhow::Result<Option<Aanvraag>> {
    let row = sqlx::query("SELECT * FROM aanvragen WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(row.as_ref().map(row_to_aanvraag))
}

pub async fn set_aanvraag_status(
    pool: &SqlitePool,
    id: &str,
    status: &str,
) -> anyhow::Result<()> {
    sqlx::query("UPDATE aanvragen SET status = ?, updated_at = datetime('now') WHERE id = ?")
        .bind(status)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn insert_besluit(
    pool: &SqlitePool,
    id: &str,
    aanvraag_id: &str,
    toegekend: bool,
    bedrag: i64,
    outputs_json: &str,
    motivering: &str,
    besluit_datum: &str,
    beoordelaar: &str,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO besluiten (id, aanvraag_id, subsidie_toegekend, subsidiebedrag, outputs, motivering, besluit_datum, beoordelaar)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(aanvraag_id)
    .bind(toegekend as i64)
    .bind(bedrag)
    .bind(outputs_json)
    .bind(motivering)
    .bind(besluit_datum)
    .bind(beoordelaar)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_besluit_by_aanvraag(
    pool: &SqlitePool,
    aanvraag_id: &str,
) -> anyhow::Result<Option<Besluit>> {
    let row = sqlx::query("SELECT * FROM besluiten WHERE aanvraag_id = ?")
        .bind(aanvraag_id)
        .fetch_optional(pool)
        .await?;
    Ok(row.as_ref().map(row_to_besluit))
}

pub async fn set_bekendmaking(
    pool: &SqlitePool,
    besluit_id: &str,
    bekendmaking_datum: &str,
    start: &str,
    eind: &str,
) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE besluiten SET bekendmaking_datum = ?, bezwaartermijn_startdatum = ?, bezwaartermijn_einddatum = ? WHERE id = ?",
    )
    .bind(bekendmaking_datum)
    .bind(start)
    .bind(eind)
    .bind(besluit_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn insert_betaalopdracht(
    pool: &SqlitePool,
    id: &str,
    besluit_id: &str,
    partij_naam: &str,
    bedrag: i64,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO betaalopdrachten (id, besluit_id, partij_naam, bedrag) VALUES (?, ?, ?, ?)",
    )
    .bind(id)
    .bind(besluit_id)
    .bind(partij_naam)
    .bind(bedrag)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_betaalopdrachten(pool: &SqlitePool) -> anyhow::Result<Vec<Betaalopdracht>> {
    let rows = sqlx::query("SELECT * FROM betaalopdrachten ORDER BY created_at DESC")
        .fetch_all(pool)
        .await?;
    Ok(rows
        .iter()
        .map(|row| Betaalopdracht {
            id: row.get("id"),
            besluit_id: row.get("besluit_id"),
            partij_naam: row.get("partij_naam"),
            bedrag: row.get("bedrag"),
            status: row.get("status"),
            created_at: row.get("created_at"),
        })
        .collect())
}

/// Public register entry: only bekendgemaakte besluiten are listed.
#[derive(Debug, Serialize)]
pub struct RegisterEntry {
    pub partij_naam: String,
    pub niveau: String,
    pub gemeente: Option<String>,
    pub subsidie_toegekend: bool,
    pub subsidiebedrag: i64,
    pub besluit_datum: String,
    pub bekendmaking_datum: String,
    pub bezwaartermijn_einddatum: Option<String>,
}

pub async fn list_register(pool: &SqlitePool) -> anyhow::Result<Vec<RegisterEntry>> {
    let rows = sqlx::query(
        "SELECT a.partij_naam, a.niveau, a.gemeente, b.subsidie_toegekend, b.subsidiebedrag,
                b.besluit_datum, b.bekendmaking_datum, b.bezwaartermijn_einddatum
         FROM besluiten b JOIN aanvragen a ON a.id = b.aanvraag_id
         WHERE b.bekendmaking_datum IS NOT NULL
         ORDER BY b.bekendmaking_datum DESC",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .iter()
        .map(|row| RegisterEntry {
            partij_naam: row.get("partij_naam"),
            niveau: row.get("niveau"),
            gemeente: row.get("gemeente"),
            subsidie_toegekend: row.get::<i64, _>("subsidie_toegekend") != 0,
            subsidiebedrag: row.get("subsidiebedrag"),
            besluit_datum: row.get("besluit_datum"),
            bekendmaking_datum: row.get("bekendmaking_datum"),
            bezwaartermijn_einddatum: row.get("bezwaartermijn_einddatum"),
        })
        .collect())
}

#[derive(Debug, Serialize)]
pub struct Statistieken {
    pub aantal_aanvragen: i64,
    pub aantal_besluiten: i64,
    pub aantal_toegekend: i64,
    pub aantal_afgewezen: i64,
    pub totaal_toegekend_bedrag: i64,
    pub per_niveau: Vec<NiveauStat>,
    pub per_maand: Vec<MaandStat>,
}

#[derive(Debug, Serialize)]
pub struct NiveauStat {
    pub niveau: String,
    pub aantal: i64,
    pub totaal_bedrag: i64,
}

#[derive(Debug, Serialize)]
pub struct MaandStat {
    pub maand: String,
    pub aantal_aanvragen: i64,
    pub toegekend_bedrag: i64,
}

pub async fn statistieken(pool: &SqlitePool) -> anyhow::Result<Statistieken> {
    let aantal_aanvragen: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM aanvragen")
        .fetch_one(pool)
        .await?;
    let aantal_besluiten: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM besluiten")
        .fetch_one(pool)
        .await?;
    let aantal_toegekend: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM besluiten WHERE subsidie_toegekend = 1")
            .fetch_one(pool)
            .await?;
    let totaal_toegekend_bedrag: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(subsidiebedrag), 0) FROM besluiten WHERE subsidie_toegekend = 1",
    )
    .fetch_one(pool)
    .await?;

    let niveau_rows = sqlx::query(
        "SELECT a.niveau AS niveau, COUNT(*) AS aantal, COALESCE(SUM(b.subsidiebedrag), 0) AS totaal
         FROM besluiten b JOIN aanvragen a ON a.id = b.aanvraag_id
         WHERE b.subsidie_toegekend = 1 GROUP BY a.niveau",
    )
    .fetch_all(pool)
    .await?;

    let maand_rows = sqlx::query(
        "SELECT strftime('%Y-%m', a.aanvraag_datum) AS maand,
                COUNT(*) AS aantal,
                COALESCE(SUM(CASE WHEN b.subsidie_toegekend = 1 THEN b.subsidiebedrag ELSE 0 END), 0) AS toegekend
         FROM aanvragen a LEFT JOIN besluiten b ON b.aanvraag_id = a.id
         GROUP BY maand ORDER BY maand",
    )
    .fetch_all(pool)
    .await?;

    Ok(Statistieken {
        aantal_aanvragen,
        aantal_besluiten,
        aantal_toegekend,
        aantal_afgewezen: aantal_besluiten - aantal_toegekend,
        totaal_toegekend_bedrag,
        per_niveau: niveau_rows
            .iter()
            .map(|r| NiveauStat {
                niveau: r.get("niveau"),
                aantal: r.get("aantal"),
                totaal_bedrag: r.get("totaal"),
            })
            .collect(),
        per_maand: maand_rows
            .iter()
            .map(|r| MaandStat {
                maand: r.get("maand"),
                aantal_aanvragen: r.get("aantal"),
                toegekend_bedrag: r.get("toegekend"),
            })
            .collect(),
    })
}
