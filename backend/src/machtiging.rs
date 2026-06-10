//! Representation scope ("machtiging") for the mocked eHerkenning login.
//!
//! eHerkenning supports (chain) authorizations: under the Wpp (consultation
//! version, explanatory memorandum to art. 27) branches of a centrally
//! organized party are organs without legal personality, but branch board
//! members can be granted a power of attorney to represent the party in a
//! limited way. This module owns the profile listing, the validation against
//! the party register and the server-side scope enforcement, so that the
//! changes in handlers.rs stay minimal.

use axum::extract::Query;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tower_sessions::Session;

use crate::db::Component;
use crate::register;

pub const SESSION_KEY_EH_MACHTIGING: &str = "eh_machtiging";

/// The representation profile an eHerkenning session carries.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum Machtiging {
    /// Signing-authorized board of the legal entity: full access.
    #[default]
    #[serde(rename = "VOLLEDIG")]
    Volledig,
    /// Branch board member with a power of attorney for one area.
    #[serde(rename = "BEPERKT")]
    Beperkt { gebied_code: String },
}

impl Machtiging {
    /// Whether a component key ("LANDELIJK" or "{orgaan}:{gebied_code}")
    /// falls within this machtiging. A limited machtiging never covers the
    /// national component.
    pub fn allows_key(&self, key: &str) -> bool {
        match self {
            Machtiging::Volledig => true,
            Machtiging::Beperkt { gebied_code } => key
                .split_once(':')
                .is_some_and(|(_, code)| code == gebied_code),
        }
    }

    /// Restrict a component list to what this machtiging covers.
    pub fn filter_componenten(&self, componenten: Vec<Component>) -> Vec<Component> {
        match self {
            Machtiging::Volledig => componenten,
            Machtiging::Beperkt { .. } => componenten
                .into_iter()
                .filter(|c| self.allows_key(&c.key))
                .collect(),
        }
    }

    /// Whether every component of an (existing) aanvraag falls within scope.
    /// Used to hide dossiers of the same legal entity that a branch board
    /// member is not authorized for.
    pub fn covers(&self, componenten: &[Component]) -> bool {
        componenten.iter().all(|c| self.allows_key(&c.key))
    }

    /// JSON for the session/status endpoint, enriched with gebied_naam and
    /// orgaan so the UI can label the active machtiging.
    pub fn to_json(&self) -> serde_json::Value {
        match self {
            Machtiging::Volledig => json!({"type": "VOLLEDIG"}),
            Machtiging::Beperkt { gebied_code } => {
                let gebied = register::gebied_by_code(gebied_code);
                json!({
                    "type": "BEPERKT",
                    "orgaan": gebied.map(|g| g.orgaan.clone()),
                    "gebied_code": gebied_code,
                    "gebied_naam": gebied
                        .map(|g| g.naam.clone())
                        .unwrap_or_else(|| gebied_code.clone()),
                })
            }
        }
    }
}

/// Validate a requested machtiging against the party register. BEPERKT is
/// only available for centrally organized parties with an election result in
/// that area; anything else can only log in as VOLLEDIG.
pub fn valideer(kvk: &str, machtiging: &Machtiging) -> Result<(), String> {
    let Machtiging::Beperkt { gebied_code } = machtiging else {
        return Ok(());
    };
    let Some(partij) = register::partij_by_kvk(kvk) else {
        return Err("Voor dit KVK-nummer zijn geen afdelingsmachtigingen beschikbaar.".to_string());
    };
    if partij.organisatiemodel != "CENTRAAL" {
        return Err(format!(
            "{} kent geen afdelingsmachtigingen: afdelingen zijn eigen rechtspersonen en loggen in met hun eigen KVK-nummer.",
            partij.naam
        ));
    }
    if register::uitslag_by_kvk_gebied(kvk, gebied_code).is_none() {
        return Err(format!(
            "{} heeft volgens het partijregister geen aanspraak in gebied '{gebied_code}'.",
            partij.naam
        ));
    }
    Ok(())
}

/// The representation profiles the mocked eHerkenning offers for a KvK
/// number: always VOLLEDIG; for centrally organized parties additionally one
/// BEPERKT profile per area with an election result (the branch volmacht).
pub fn profielen_voor(kvk: &str) -> Vec<serde_json::Value> {
    let mut profielen = vec![json!({
        "type": "VOLLEDIG",
        "omschrijving": "De gehele partij (tekenbevoegd bestuur)",
    })];
    let Some(partij) = register::partij_by_kvk(kvk) else {
        return profielen;
    };
    if partij.organisatiemodel != "CENTRAAL" {
        return profielen;
    }
    for u in &partij.decentrale_uitslagen {
        let gebied = register::gebied_by_code(&u.gebied_code);
        profielen.push(json!({
            "type": "BEPERKT",
            "orgaan": u.orgaan,
            "gebied_code": u.gebied_code,
            "gebied_naam": gebied
                .map(|g| g.naam.clone())
                .unwrap_or_else(|| u.gebied_code.clone()),
        }));
    }
    profielen
}

#[derive(Deserialize)]
pub struct MachtigingenQuery {
    pub kvk: String,
}

/// GET /api/eherkenning/machtigingen?kvk=... — which representation profiles
/// the mocked eHerkenning would offer for this legal entity. Unknown KvK
/// numbers get only VOLLEDIG (anyone may apply, AWB 4:1).
pub async fn machtigingen(Query(q): Query<MachtigingenQuery>) -> Json<serde_json::Value> {
    let kvk = q.kvk.trim();
    Json(json!({
        "kvk_nummer": kvk,
        "partij_naam": register::partij_by_kvk(kvk).map(|p| p.naam.clone()),
        "profielen": profielen_voor(kvk),
    }))
}

/// Read the machtiging from the session. Absent (sessions from before this
/// feature, or a login without the field) means VOLLEDIG — backwards
/// compatible with the existing BDD suite and seed script.
pub async fn session_machtiging(session: &Session) -> Machtiging {
    session
        .get::<Machtiging>(SESSION_KEY_EH_MACHTIGING)
        .await
        .ok()
        .flatten()
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Fixed entries in the register snapshot used by the demo scenario.
    const D66: &str = "92525446"; // CENTRAAL, incl. GEMEENTERAAD:GM0344 (Utrecht)
    const CDA_AFDELING: &str = "99399789"; // DECENTRAAL (own legal entity)
    const ONBEKEND: &str = "00000000";

    fn beperkt(code: &str) -> Machtiging {
        Machtiging::Beperkt {
            gebied_code: code.to_string(),
        }
    }

    fn component(key: &str) -> Component {
        Component {
            key: key.to_string(),
            soort: if key == "LANDELIJK" {
                "LANDELIJK".into()
            } else {
                "DECENTRAAL".into()
            },
            orgaan: None,
            gebied_code: None,
            gebied: None,
            zetels: 1,
            inwoneraantal: 0,
        }
    }

    #[test]
    fn volledig_allows_every_key() {
        let m = Machtiging::Volledig;
        assert!(m.allows_key("LANDELIJK"));
        assert!(m.allows_key("GEMEENTERAAD:GM0344"));
        assert!(m.allows_key("PROVINCIALE_STATEN:PV26"));
    }

    #[test]
    fn beperkt_allows_only_own_gebied_and_never_landelijk() {
        let m = beperkt("GM0344");
        assert!(m.allows_key("GEMEENTERAAD:GM0344"));
        assert!(!m.allows_key("LANDELIJK"));
        assert!(!m.allows_key("GEMEENTERAAD:GM0034"));
        assert!(!m.allows_key("PROVINCIALE_STATEN:PV26"));
    }

    #[test]
    fn filter_componenten_keeps_only_scope() {
        let alles = vec![
            component("LANDELIJK"),
            component("GEMEENTERAAD:GM0344"),
            component("GEMEENTERAAD:GM0599"),
        ];
        let gefilterd = beperkt("GM0344").filter_componenten(alles.clone());
        assert_eq!(gefilterd.len(), 1);
        assert_eq!(gefilterd[0].key, "GEMEENTERAAD:GM0344");
        assert_eq!(Machtiging::Volledig.filter_componenten(alles).len(), 3);
    }

    #[test]
    fn covers_requires_all_components_in_scope() {
        let m = beperkt("GM0344");
        assert!(m.covers(&[component("GEMEENTERAAD:GM0344")]));
        assert!(!m.covers(&[component("GEMEENTERAAD:GM0344"), component("LANDELIJK")]));
        assert!(Machtiging::Volledig.covers(&[component("LANDELIJK")]));
        // An empty aanvraag cannot exist, but covers should not panic.
        assert!(m.covers(&[]));
    }

    #[test]
    fn valideer_volledig_is_always_ok() {
        assert!(valideer(D66, &Machtiging::Volledig).is_ok());
        assert!(valideer(ONBEKEND, &Machtiging::Volledig).is_ok());
    }

    #[test]
    fn valideer_beperkt_requires_centraal_party_with_uitslag() {
        assert!(valideer(D66, &beperkt("GM0344")).is_ok());
        // Unknown legal entity: no branch machtigingen.
        assert!(valideer(ONBEKEND, &beperkt("GM0344")).is_err());
        // Decentrally organized: branches are their own legal entity.
        assert!(valideer(CDA_AFDELING, &beperkt("GM0518")).is_err());
        // Centrally organized but no result in that area.
        assert!(valideer(D66, &beperkt("XX9999")).is_err());
    }

    #[test]
    fn profielen_volledig_plus_one_per_gebied_for_centraal() {
        let onbekend = profielen_voor(ONBEKEND);
        assert_eq!(onbekend.len(), 1);
        assert_eq!(onbekend[0]["type"], "VOLLEDIG");

        let afdeling = profielen_voor(CDA_AFDELING);
        assert_eq!(afdeling.len(), 1, "DECENTRAAL gets no branch profiles");

        let d66 = profielen_voor(D66);
        assert!(d66.len() > 1);
        assert_eq!(d66[0]["type"], "VOLLEDIG");
        let utrecht = d66
            .iter()
            .find(|p| p["gebied_code"] == "GM0344")
            .expect("D66 has a profile for Utrecht");
        assert_eq!(utrecht["type"], "BEPERKT");
        assert_eq!(utrecht["orgaan"], "GEMEENTERAAD");
        assert_eq!(utrecht["gebied_naam"], "Utrecht");
    }

    #[test]
    fn machtiging_deserializes_from_login_payload() {
        let volledig: Machtiging = serde_json::from_value(json!({"type": "VOLLEDIG"})).unwrap();
        assert_eq!(volledig, Machtiging::Volledig);
        let beperkt_m: Machtiging =
            serde_json::from_value(json!({"type": "BEPERKT", "gebied_code": "GM0344"})).unwrap();
        assert_eq!(beperkt_m, beperkt("GM0344"));
        assert!(serde_json::from_value::<Machtiging>(json!({"type": "BEPERKT"})).is_err());
    }

    #[test]
    fn to_json_enriches_with_gebied_naam() {
        let v = beperkt("GM0344").to_json();
        assert_eq!(v["type"], "BEPERKT");
        assert_eq!(v["gebied_naam"], "Utrecht");
        assert_eq!(v["orgaan"], "GEMEENTERAAD");
        assert_eq!(Machtiging::Volledig.to_json(), json!({"type": "VOLLEDIG"}));
    }
}
