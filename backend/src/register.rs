//! Partijregister van de Napp, gevoed uit open data.
//!
//! De snapshot in `data/partijregister.json` wordt gegenereerd door
//! `scripts/bouw_register.py` uit drie open databronnen: de
//! verkiezingsuitslag Tweede Kamer 2025 en de gemeenteraadsuitslagen 2026
//! (Kiesraad, data.overheid.nl) en de inwoneraantallen per gemeente
//! (CBS StatLine). De KvK-nummers zijn synthetisch: de koppeling
//! rechtspersoon-aanduiding is geen open data en is precies wat de Napp
//! bij registratie vastlegt.
//!
//! Het datamodel kent decentrale organen GEMEENTERAAD, PROVINCIALE_STATEN
//! en WATERSCHAP; provinciale staten en waterschappen worden gevuld zodra
//! de zetel-import daarvoor bestaat (zie het buildscript).
//!
//! Een onbekend KvK-nummer mag gewoon een aanvraag indienen (AWB 4:1) —
//! de wet wijst dan af. Het register sluit de aanvraagroute niet af.

use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Partij {
    pub kvk_nummer: String,
    pub naam: String,
    /// CENTRAAL (afdelingen onder een KvK) of DECENTRAAL (afdelingen als
    /// eigen rechtspersoon) — Wpp-organisatiemodellen.
    #[serde(default)]
    pub organisatiemodel: String,
    /// Zetels in Eerste + Tweede Kamer (bron: Kiesraad, TK2025).
    pub kamerzetels: i64,
    #[serde(default)]
    pub moederpartij_kvk: Option<String>,
    pub decentrale_uitslagen: Vec<Uitslag>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Uitslag {
    /// GEMEENTERAAD | PROVINCIALE_STATEN | WATERSCHAP
    pub orgaan: String,
    pub gebied_code: String,
    pub zetels: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Gebied {
    pub orgaan: String,
    pub code: String,
    pub naam: String,
    /// Bron: CBS StatLine.
    pub inwoneraantal: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DemoVoorbeeld {
    pub kvk_nummer: String,
    pub naam: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Register {
    pub partijen: Vec<Partij>,
    pub gebieden: Vec<Gebied>,
    pub demo_voorbeelden: Vec<DemoVoorbeeld>,
}

static REGISTER: OnceLock<Register> = OnceLock::new();

pub fn register() -> &'static Register {
    REGISTER.get_or_init(|| {
        serde_json::from_str(include_str!("../data/partijregister.json"))
            .expect("partijregister.json is geen geldig register")
    })
}

pub fn partij_by_kvk(kvk: &str) -> Option<&'static Partij> {
    register().partijen.iter().find(|p| p.kvk_nummer == kvk)
}

pub fn gebied_by_code(code: &str) -> Option<&'static Gebied> {
    register().gebieden.iter().find(|g| g.code == code)
}

pub fn uitslag_by_kvk_gebied(kvk: &str, gebied_code: &str) -> Option<&'static Uitslag> {
    partij_by_kvk(kvk)?
        .decentrale_uitslagen
        .iter()
        .find(|u| u.gebied_code == gebied_code)
}
