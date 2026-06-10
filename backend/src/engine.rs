//! Wrapper around the regelrecht engine.
//!
//! De wet rekent per aanspraak (artikel 8 landelijk, artikel 12 decentraal);
//! de orchestratielaag voert de wet per component uit en telt de bedragen
//! op tot één beschikking. Die som is bewust orchestratie en geen wet: de
//! engine kent geen aggregatie over collecties (RFC-012).

use std::collections::BTreeMap;
use std::sync::Arc;

use regelrecht_engine::{LawExecutionService, Value};
use serde::Serialize;

use crate::db::{Component, ComponentUitkomst, LandelijkeDelen};
use crate::state::LawCorpus;

pub const WPP_ID: &str = "wet_op_de_politieke_partijen";
pub const AWB_ID: &str = "algemene_wet_bestuursrecht";
pub const ATW_ID: &str = "algemene_termijnenwet";

/// Uitkomst van een samengestelde jaaraanvraag.
#[derive(Debug, Clone, Serialize)]
pub struct JaaruitkomstUitkomst {
    pub subsidie_toegekend: bool,
    pub subsidiebedrag: i64,
    pub betaalopdracht_vereist: bool,
    pub betaalopdracht_bedrag: i64,
    pub bezwaartermijn_weken: i64,
    pub motivering_vereist: bool,
    pub voldoet_aan_transparantie: bool,
    pub componenten: Vec<ComponentUitkomst>,
    pub motivering: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BezwaartermijnUitkomst {
    pub startdatum: String,
    pub einddatum: String,
}

fn service_with_corpus(corpus: &LawCorpus) -> anyhow::Result<LawExecutionService> {
    let mut service = LawExecutionService::new();
    service
        .load_law(&corpus.wpp)
        .map_err(|e| anyhow::anyhow!("laden Wpp mislukt: {e}"))?;
    service
        .load_law(&corpus.regeling)
        .map_err(|e| anyhow::anyhow!("laden regeling mislukt: {e}"))?;
    service
        .load_law(&corpus.besluit_decentraal)
        .map_err(|e| anyhow::anyhow!("laden besluit decentrale subsidies mislukt: {e}"))?;
    service
        .load_law(&corpus.awb)
        .map_err(|e| anyhow::anyhow!("laden AWB mislukt: {e}"))?;
    service
        .load_law(&corpus.termijnenwet)
        .map_err(|e| anyhow::anyhow!("laden Algemene termijnenwet mislukt: {e}"))?;
    Ok(service)
}

fn as_bool(outputs: &BTreeMap<String, Value>, name: &str) -> bool {
    matches!(outputs.get(name), Some(Value::Bool(true)))
}

fn as_int(outputs: &BTreeMap<String, Value>, name: &str) -> i64 {
    match outputs.get(name) {
        Some(Value::Int(n)) => *n,
        Some(Value::Float(f)) => f.round() as i64,
        _ => 0,
    }
}

fn euro(cents: i64) -> String {
    let negative = cents < 0;
    let cents = cents.abs();
    let whole = cents / 100;
    let rest = cents % 100;
    let mut s = whole.to_string();
    let mut grouped = String::new();
    while s.len() > 3 {
        let tail = s.split_off(s.len() - 3);
        grouped = format!(".{tail}{grouped}");
    }
    let sign = if negative { "-" } else { "" };
    format!("{sign}€ {s}{grouped},{rest:02}")
}

/// Engine-parameters voor één component, gecombineerd met de eigen opgaven
/// (ledental, neveninstellingen, transparantieverklaringen) die op
/// rechtspersoonsniveau gelden, plus het door de orchestratie aangeleverde
/// ledentotaal (de noemer van de ledencomponent van art. 14).
fn component_params(
    component: &Component,
    eigen: &serde_json::Map<String, serde_json::Value>,
    totaal_leden: i64,
) -> BTreeMap<String, Value> {
    let mut params: BTreeMap<String, Value> = BTreeMap::new();
    for key in [
        "ontvangt_anonieme_giften",
        "ontvangt_giften_niet_ingezetenen",
        "voldoet_aan_meldplicht_giften",
        "financien_openbaar_op_website",
    ] {
        let waarde = eigen.get(key).and_then(|v| v.as_bool()).unwrap_or(false);
        params.insert(key.to_string(), Value::Bool(waarde));
    }
    let leden = eigen
        .get("aantal_betalende_leden")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let eigen_bool = |key: &str| eigen.get(key).and_then(|v| v.as_bool()).unwrap_or(false);
    let pjo_leden = eigen
        .get("aantal_leden_jongerenorganisatie")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    if component.soort == "LANDELIJK" {
        params.insert("niveau".into(), Value::String("LANDELIJK".into()));
        params.insert("orgaan".into(), Value::String("GEMEENTERAAD".into()));
        params.insert("aantal_kamerzetels".into(), Value::Int(component.zetels));
        params.insert("aantal_betalende_leden".into(), Value::Int(leden));
        params.insert(
            "totaal_aantal_betalende_leden".into(),
            Value::Int(totaal_leden),
        );
        params.insert(
            "heeft_wetenschappelijk_instituut".into(),
            Value::Bool(eigen_bool("heeft_wetenschappelijk_instituut")),
        );
        params.insert(
            "heeft_jongerenorganisatie".into(),
            Value::Bool(eigen_bool("heeft_jongerenorganisatie")),
        );
        params.insert(
            "aantal_leden_jongerenorganisatie".into(),
            Value::Int(pjo_leden),
        );
        params.insert(
            "heeft_instelling_buitenland".into(),
            Value::Bool(eigen_bool("heeft_instelling_buitenland")),
        );
        params.insert("aantal_raadszetels".into(), Value::Int(0));
        params.insert("inwoneraantal_gemeente".into(), Value::Int(0));
    } else {
        params.insert("niveau".into(), Value::String("DECENTRAAL".into()));
        params.insert(
            "orgaan".into(),
            Value::String(component.orgaan.clone().unwrap_or_else(|| "GEMEENTERAAD".into())),
        );
        params.insert("aantal_raadszetels".into(), Value::Int(component.zetels));
        params.insert(
            "inwoneraantal_gemeente".into(),
            Value::Int(component.inwoneraantal),
        );
        params.insert("aantal_kamerzetels".into(), Value::Int(0));
        params.insert("aantal_betalende_leden".into(), Value::Int(0));
        params.insert("totaal_aantal_betalende_leden".into(), Value::Int(0));
        params.insert(
            "heeft_wetenschappelijk_instituut".into(),
            Value::Bool(false),
        );
        params.insert("heeft_jongerenorganisatie".into(), Value::Bool(false));
        params.insert("aantal_leden_jongerenorganisatie".into(), Value::Int(0));
        params.insert("heeft_instelling_buitenland".into(), Value::Bool(false));
    }
    params
}

fn orgaan_label(component: &Component) -> String {
    match component.soort.as_str() {
        "LANDELIJK" => "landelijk".to_string(),
        _ => match component.orgaan.as_deref() {
            Some("PROVINCIALE_STATEN") => {
                format!("provinciale staten {}", component.gebied.as_deref().unwrap_or("?"))
            }
            Some("WATERSCHAP") => {
                format!("waterschap {}", component.gebied.as_deref().unwrap_or("?"))
            }
            Some("EILANDSRAAD") => {
                format!("eilandsraad {}", component.gebied.as_deref().unwrap_or("?"))
            }
            _ => format!("gemeenteraad {}", component.gebied.as_deref().unwrap_or("?")),
        },
    }
}

fn build_motivering(
    eigen: &serde_json::Map<String, serde_json::Value>,
    voldoet_transparantie: bool,
    uitkomsten: &[ComponentUitkomst],
    totaal: i64,
    subsidiejaar: i64,
    voorschot: i64,
) -> String {
    let toegekend: Vec<_> = uitkomsten.iter().filter(|u| u.toegekend).collect();
    let afgewezen: Vec<_> = uitkomsten.iter().filter(|u| !u.toegekend).collect();

    let mut delen: Vec<String> = Vec::new();

    if !voldoet_transparantie {
        let mut redenen = Vec::new();
        let b = |k: &str| eigen.get(k).and_then(|v| v.as_bool());
        if b("ontvangt_anonieme_giften") == Some(true) {
            redenen.push("de partij ontvangt anonieme giften");
        }
        if b("ontvangt_giften_niet_ingezetenen") == Some(true) {
            redenen.push("de partij ontvangt giften van niet-ingezetenen");
        }
        if b("voldoet_aan_meldplicht_giften") == Some(false) {
            redenen.push("de partij voldoet niet aan de meldplicht voor giften van € 10.000 of meer");
        }
        if b("financien_openbaar_op_website") == Some(false) {
            redenen.push("de partij maakt haar financiën niet openbaar op haar website");
        }
        delen.push(format!(
            "De aanvraag wordt afgewezen omdat niet aan de transparantie-eisen van artikel 5 \
             van de Wet op de politieke partijen is voldaan: {}.",
            redenen.join("; ")
        ));
        return delen.join(" ");
    }

    delen.push(
        "De aanvraag voldoet aan de transparantie-eisen van artikel 5 van de Wet op de \
         politieke partijen."
            .to_string(),
    );

    if !toegekend.is_empty() {
        let n = toegekend.len();
        delen.push(format!(
            "Op grond van de artikelen 6, 7, 12 en 14 wordt voor subsidiejaar {subsidiejaar} \
             een subsidie verleend van {} voor {n} {}, overeenkomstig de specificatie bij \
             dit besluit. Op grond van artikel 17, derde lid, wordt van rechtswege een \
             voorschot verleend van tachtig procent: {}.",
            euro(totaal),
            if n == 1 { "onderdeel" } else { "onderdelen" },
            euro(voorschot)
        ));
    }
    for u in &afgewezen {
        if u.component.soort == "LANDELIJK" {
            let leden = eigen
                .get("aantal_betalende_leden")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let reden = if u.component.zetels < 1 {
                "de partij geen zetel in de Eerste of Tweede Kamer heeft"
            } else if leden < 1000 {
                "de partij minder dan duizend betalende leden heeft"
            } else {
                "niet aan de voorwaarden van artikel 6 is voldaan"
            };
            delen.push(format!(
                "Het landelijke onderdeel wordt afgewezen omdat {reden} (artikel 6)."
            ));
        } else {
            delen.push(format!(
                "Het onderdeel {} wordt afgewezen omdat daar geen zetel is toegewezen (artikel 7).",
                orgaan_label(&u.component)
            ));
        }
    }
    if toegekend.is_empty() && afgewezen.is_empty() {
        delen.push("De aanvraag bevat geen onderdelen.".to_string());
    }
    delen.join(" ")
}

/// Voer de wet uit voor elke component van een jaaraanvraag en stel het
/// samengestelde verleningsbesluit op. `totaal_leden` is de noemer van de
/// ledencomponent (art. 14, onderdeel a): de som van de opgegeven
/// ledentallen van alle aanvragen voor het subsidiejaar — aggregatie over
/// aanvragen is bewust orchestratie en geen wet (RFC-012).
pub async fn evaluate_jaaraanvraag(
    corpus: Arc<LawCorpus>,
    componenten: Vec<Component>,
    eigen: serde_json::Map<String, serde_json::Value>,
    date: String,
    subsidiejaar: i64,
    totaal_leden: i64,
) -> anyhow::Result<JaaruitkomstUitkomst> {
    tokio::task::spawn_blocking(move || {
        let service = service_with_corpus(&corpus)?;

        let mut uitkomsten: Vec<ComponentUitkomst> = Vec::new();
        let mut totaal: i64 = 0;
        let mut betaal_totaal: i64 = 0;
        let mut bezwaartermijn_weken = 0;
        let mut motivering_vereist = false;
        let mut voldoet_transparantie = true;

        for component in &componenten {
            let params = component_params(component, &eigen, totaal_leden);
            let result = service
                .evaluate_law(
                    WPP_ID,
                    &["subsidie_toegekend", "subsidiebedrag"],
                    params.clone(),
                    &date,
                )
                .map_err(|e| {
                    anyhow::anyhow!(
                        "berekening onderdeel {} mislukt: {e}",
                        component.key
                    )
                })?;

            let toegekend = as_bool(&result.outputs, "subsidie_toegekend");
            let bedrag = as_int(&result.outputs, "subsidiebedrag");
            totaal += bedrag;
            betaal_totaal += as_int(&result.outputs, "betaalopdracht_bedrag");
            bezwaartermijn_weken = as_int(&result.outputs, "bezwaartermijn_weken");
            motivering_vereist |= as_bool(&result.outputs, "motivering_vereist");

            // De specificatie van de landelijke component: de vier delen van
            // art. 14 (partij + geoormerkte bedragen neveninstellingen).
            let delen = if component.soort == "LANDELIJK" && toegekend {
                let delen_result = service
                    .evaluate_law(
                        WPP_ID,
                        &[
                            "subsidie_partij",
                            "subsidie_wetenschappelijk_instituut",
                            "subsidie_jongerenorganisatie",
                            "subsidie_buitenland",
                        ],
                        params.clone(),
                        &date,
                    )
                    .map_err(|e| anyhow::anyhow!("specificatie art. 14 mislukt: {e}"))?;
                Some(LandelijkeDelen {
                    partij: as_int(&delen_result.outputs, "subsidie_partij"),
                    wetenschappelijk_instituut: as_int(
                        &delen_result.outputs,
                        "subsidie_wetenschappelijk_instituut",
                    ),
                    jongerenorganisatie: as_int(
                        &delen_result.outputs,
                        "subsidie_jongerenorganisatie",
                    ),
                    buitenland: as_int(&delen_result.outputs, "subsidie_buitenland"),
                })
            } else {
                None
            };

            uitkomsten.push(ComponentUitkomst {
                component: component.clone(),
                toegekend,
                bedrag,
                delen,
            });
        }

        // Transparantie geldt op rechtspersoonsniveau; één toets volstaat.
        if let Some(component) = componenten.first() {
            let checks = service
                .evaluate_law(
                    WPP_ID,
                    &["voldoet_aan_transparantie"],
                    component_params(component, &eigen, totaal_leden),
                    &date,
                )
                .map_err(|e| anyhow::anyhow!("toetsing transparantie mislukt: {e}"))?;
            voldoet_transparantie = as_bool(&checks.outputs, "voldoet_aan_transparantie");
        }

        let toegekend = uitkomsten.iter().any(|u| u.toegekend);
        let motivering = build_motivering(
            &eigen,
            voldoet_transparantie,
            &uitkomsten,
            totaal,
            subsidiejaar,
            betaal_totaal,
        );

        Ok(JaaruitkomstUitkomst {
            subsidie_toegekend: toegekend,
            subsidiebedrag: totaal,
            betaalopdracht_vereist: betaal_totaal > 0,
            betaalopdracht_bedrag: betaal_totaal,
            bezwaartermijn_weken,
            motivering_vereist,
            voldoet_aan_transparantie: voldoet_transparantie,
            componenten: uitkomsten,
            motivering,
        })
    })
    .await?
}

/// Termijnen van de jaarcyclus (Wpp art. 17, lex specialis t.o.v. AWB 4:13):
/// de aanvraag moet uiterlijk 1 november voorafgaand aan het subsidiejaar
/// binnen zijn en de Napp besluit voor 1 januari van het subsidiejaar. De
/// beslistermijn is een uiterste datum gekoppeld aan de start van het
/// subsidiejaar; de Algemene termijnenwet wordt hier bewust niet toegepast,
/// omdat verlenging voorbij 1 januari het doel van de termijn (besluit vóór
/// aanvang van het subsidiejaar) zou tenietdoen.
#[derive(Debug, Clone, Serialize)]
pub struct TermijnenUitkomst {
    pub aanvraagtermijn_einddatum: String,
    pub beslistermijn_einddatum: String,
    pub voorschotpercentage: i64,
}

pub async fn evaluate_termijnen(
    corpus: Arc<LawCorpus>,
    subsidiejaar: i64,
    date: String,
) -> anyhow::Result<TermijnenUitkomst> {
    tokio::task::spawn_blocking(move || {
        let service = service_with_corpus(&corpus)?;
        let mut params = BTreeMap::new();
        params.insert(
            "subsidiejaar_startdatum".to_string(),
            Value::String(format!("{subsidiejaar}-01-01")),
        );
        let result = service
            .evaluate_law(
                WPP_ID,
                &[
                    "aanvraagtermijn_einddatum",
                    "beslistermijn_einddatum",
                    "voorschotpercentage",
                ],
                params,
                &date,
            )
            .map_err(|e| anyhow::anyhow!("termijnen art. 17 berekenen mislukt: {e}"))?;
        let as_date = |name: &str| -> String {
            match result.outputs.get(name) {
                Some(Value::String(s)) => s.clone(),
                other => format!("{other:?}"),
            }
        };
        Ok(TermijnenUitkomst {
            aanvraagtermijn_einddatum: as_date("aanvraagtermijn_einddatum"),
            beslistermijn_einddatum: as_date("beslistermijn_einddatum"),
            voorschotpercentage: as_int(&result.outputs, "voorschotpercentage"),
        })
    })
    .await?
}

/// Compute the bezwaartermijn dates after bekendmaking (AWB 6:8), with the
/// weekend extension from the Algemene termijnenwet (art. 1).
pub async fn evaluate_bezwaartermijn(
    corpus: Arc<LawCorpus>,
    bekendmaking_datum: String,
) -> anyhow::Result<BezwaartermijnUitkomst> {
    tokio::task::spawn_blocking(move || {
        let service = service_with_corpus(&corpus)?;
        let mut params = BTreeMap::new();
        params.insert(
            "bekendmaking_datum".to_string(),
            Value::String(bekendmaking_datum.clone()),
        );
        let result = service
            .evaluate_law(
                AWB_ID,
                &["bezwaartermijn_startdatum", "bezwaartermijn_einddatum"],
                params,
                &bekendmaking_datum,
            )
            .map_err(|e| anyhow::anyhow!("bezwaartermijn berekenen mislukt: {e}"))?;
        let as_date = |outputs: &BTreeMap<String, Value>, name: &str| -> String {
            match outputs.get(name) {
                Some(Value::String(s)) => s.clone(),
                other => format!("{other:?}"),
            }
        };
        let einddatum = as_date(&result.outputs, "bezwaartermijn_einddatum");

        let mut atw_params = BTreeMap::new();
        atw_params.insert("einddatum".to_string(), Value::String(einddatum.clone()));
        let verlengd = service
            .evaluate_law(
                ATW_ID,
                &["verlengde_einddatum"],
                atw_params,
                &bekendmaking_datum,
            )
            .map_err(|e| anyhow::anyhow!("termijnverlenging berekenen mislukt: {e}"))?;

        Ok(BezwaartermijnUitkomst {
            startdatum: as_date(&result.outputs, "bezwaartermijn_startdatum"),
            einddatum: as_date(&verlengd.outputs, "verlengde_einddatum"),
        })
    })
    .await?
}
