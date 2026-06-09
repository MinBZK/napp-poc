//! Partijregister van de Napp (mock).
//!
//! In het echte stelsel registreert de Napp politieke partijen (Wpp) en
//! koppelt zij de rechtspersoon (KvK, via eHerkenning) aan de geregistreerde
//! aanduiding (Kieswet G-1). De zetelaantallen komen uit de uitslagen van de
//! Kiesraad; inwoneraantallen uit de CBS/BRP-bevolkingscijfers. Dit bestand
//! mockt die bronnen met een vaste dataset.
//!
//! Belangrijk ontwerpprincipe: een onbekend KvK-nummer mag gewoon een
//! aanvraag indienen (AWB 4:1) — de wet wijst dan af. Het register sluit de
//! aanvraagroute niet af; het levert alleen de officiële gegevens.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Partij {
    pub kvk_nummer: &'static str,
    pub naam: &'static str,
    /// Zetels in Eerste + Tweede Kamer (bron: Kiesraad).
    pub kamerzetels: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DecentraleUitslag {
    pub kvk_nummer: &'static str,
    pub gemeente: &'static str,
    /// Behaalde raadszetels bij de laatste raadsverkiezing (bron: Kiesraad).
    pub raadszetels: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Gemeente {
    pub naam: &'static str,
    /// Inwoneraantal (bron: CBS).
    pub inwoneraantal: i64,
}

/// Geregistreerde partijen (fictief).
pub const PARTIJEN: &[Partij] = &[
    Partij { kvk_nummer: "87654321", naam: "Vrijheid en Vooruitgang", kamerzetels: 12 },
    Partij { kvk_nummer: "23456789", naam: "Algemene Volkspartij", kamerzetels: 28 },
    Partij { kvk_nummer: "34567890", naam: "Partij voor Stad en Land", kamerzetels: 0 },
    Partij { kvk_nummer: "45678901", naam: "Nieuw Geluid", kamerzetels: 3 },
];

/// Decentrale uitslagen (fictief).
pub const DECENTRALE_UITSLAGEN: &[DecentraleUitslag] = &[
    DecentraleUitslag { kvk_nummer: "34567890", gemeente: "Utrecht", raadszetels: 7 },
    DecentraleUitslag { kvk_nummer: "34567890", gemeente: "Lopik", raadszetels: 3 },
    DecentraleUitslag { kvk_nummer: "45678901", gemeente: "Amsterdam", raadszetels: 2 },
    DecentraleUitslag { kvk_nummer: "87654321", gemeente: "Amersfoort", raadszetels: 4 },
];

/// Gemeenten met inwoneraantal (fictieve cijfers in echte ordes van grootte).
pub const GEMEENTEN: &[Gemeente] = &[
    Gemeente { naam: "Amsterdam", inwoneraantal: 905_000 },
    Gemeente { naam: "Utrecht", inwoneraantal: 368_000 },
    Gemeente { naam: "Amersfoort", inwoneraantal: 160_000 },
    Gemeente { naam: "Lopik", inwoneraantal: 14_500 },
];

pub fn partij_by_kvk(kvk: &str) -> Option<&'static Partij> {
    PARTIJEN.iter().find(|p| p.kvk_nummer == kvk)
}

pub fn decentrale_uitslagen_by_kvk(kvk: &str) -> Vec<&'static DecentraleUitslag> {
    DECENTRALE_UITSLAGEN
        .iter()
        .filter(|u| u.kvk_nummer == kvk)
        .collect()
}

pub fn uitslag_by_kvk_gemeente(kvk: &str, gemeente: &str) -> Option<&'static DecentraleUitslag> {
    DECENTRALE_UITSLAGEN
        .iter()
        .find(|u| u.kvk_nummer == kvk && u.gemeente == gemeente)
}

pub fn gemeente_by_naam(naam: &str) -> Option<&'static Gemeente> {
    GEMEENTEN.iter().find(|g| g.naam == naam)
}
