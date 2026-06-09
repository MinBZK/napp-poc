//! Wrapper around the regelrecht engine: builds a fresh `LawExecutionService`
//! per evaluation (the service is not Sync; construction from cached YAML is
//! cheap) and exposes the two evaluations the orchestration layer needs.

use std::collections::BTreeMap;
use std::sync::Arc;

use regelrecht_engine::{LawExecutionService, Value};
use serde::Serialize;

use crate::state::LawCorpus;

pub const WPP_ID: &str = "wet_op_de_politieke_partijen";
pub const AWB_ID: &str = "algemene_wet_bestuursrecht";

/// Outcome of the besluit evaluation, including reactive hook outputs and
/// the intermediate checks used for the motivering.
#[derive(Debug, Clone, Serialize)]
pub struct BesluitUitkomst {
    pub subsidie_toegekend: bool,
    pub subsidiebedrag: i64,
    pub betaalopdracht_vereist: bool,
    pub betaalopdracht_bedrag: i64,
    pub bezwaartermijn_weken: i64,
    pub motivering_vereist: bool,
    pub voldoet_aan_transparantie: bool,
    pub heeft_recht_landelijk: bool,
    pub heeft_recht_decentraal: bool,
    /// All raw outputs (incl. provenance-bearing hook outputs), JSON-encoded.
    pub outputs: serde_json::Value,
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
        .load_law(&corpus.awb)
        .map_err(|e| anyhow::anyhow!("laden AWB mislukt: {e}"))?;
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
    // Dutch thousands separator
    let mut s = whole.to_string();
    let mut grouped = String::new();
    while s.len() > 3 {
        let tail = s.split_off(s.len() - 3);
        grouped = format!(".{tail}{grouped}");
    }
    let sign = if negative { "-" } else { "" };
    format!("{sign}€ {s}{grouped},{rest:02}")
}

fn build_motivering(
    params: &BTreeMap<String, Value>,
    toegekend: bool,
    bedrag: i64,
    voldoet_transparantie: bool,
    niveau: &str,
) -> String {
    if toegekend {
        let grondslag = if niveau == "LANDELIJK" {
            "artikel 6 en 8 van de Wet op de politieke partijen (kamerzetels, ledental en de bij \
             ministeriële regeling vastgestelde bedragen)"
        } else {
            "artikel 7 en 12 van de Wet op de politieke partijen (raadszetels en het bij \
             ministeriële regeling vastgestelde bedrag per zetel naar inwoneraantal)"
        };
        return format!(
            "De aanvraag voldoet aan de transparantie-eisen van artikel 5 van de Wet op de \
             politieke partijen. Op grond van {grondslag} wordt de subsidie vastgesteld op {}.",
            euro(bedrag)
        );
    }

    let mut redenen: Vec<String> = Vec::new();
    if !voldoet_transparantie {
        if matches!(params.get("ontvangt_anonieme_giften"), Some(Value::Bool(true))) {
            redenen.push(
                "de partij ontvangt anonieme giften, hetgeen op grond van artikel 5 verboden is"
                    .to_string(),
            );
        }
        if matches!(
            params.get("ontvangt_giften_niet_ingezetenen"),
            Some(Value::Bool(true))
        ) {
            redenen.push(
                "de partij ontvangt giften van niet-ingezetenen, hetgeen op grond van artikel 5 \
                 verboden is"
                    .to_string(),
            );
        }
        if matches!(
            params.get("voldoet_aan_meldplicht_giften"),
            Some(Value::Bool(false))
        ) {
            redenen.push(
                "de partij voldoet niet aan de meldplicht voor giften van € 10.000 of meer \
                 (artikel 5)"
                    .to_string(),
            );
        }
        if matches!(
            params.get("financien_openbaar_op_website"),
            Some(Value::Bool(false))
        ) {
            redenen.push(
                "de partij maakt haar financiën niet openbaar op haar website (artikel 5)"
                    .to_string(),
            );
        }
    }
    if niveau == "LANDELIJK" {
        if as_int(params, "aantal_kamerzetels") < 1 {
            redenen.push(
                "de partij heeft geen zetel in de Eerste of Tweede Kamer (artikel 6)".to_string(),
            );
        }
        if as_int(params, "aantal_betalende_leden") < 1000 {
            redenen.push(
                "de partij heeft minder dan duizend betalende leden (artikel 6)".to_string(),
            );
        }
    } else if as_int(params, "aantal_raadszetels") < 1 {
        redenen.push(
            "de partij heeft geen zetel behaald bij de laatstgehouden decentrale verkiezing \
             (artikel 7)"
                .to_string(),
        );
    }
    if redenen.is_empty() {
        redenen.push("de aanvraag voldoet niet aan de wettelijke voorwaarden".to_string());
    }
    format!(
        "De aanvraag wordt afgewezen omdat {}.",
        redenen.join("; en omdat ")
    )
}

/// Evaluate the subsidiebesluit for the given application parameters.
///
/// Runs two evaluations: the intermediate checks (art. 5/6/7) for the
/// motivering, and the besluit itself (art. 15) which reactively fires the
/// betaalopdracht hook (art. 16) and the AWB hooks (3:46, 6:7).
pub async fn evaluate_besluit(
    corpus: Arc<LawCorpus>,
    params: BTreeMap<String, Value>,
    date: String,
) -> anyhow::Result<BesluitUitkomst> {
    tokio::task::spawn_blocking(move || {
        let service = service_with_corpus(&corpus)?;

        let checks = service
            .evaluate_law(
                WPP_ID,
                &[
                    "voldoet_aan_transparantie",
                    "heeft_recht_landelijk",
                    "heeft_recht_decentraal",
                ],
                params.clone(),
                &date,
            )
            .map_err(|e| anyhow::anyhow!("toetsing voorwaarden mislukt: {e}"))?;

        // Request only the besluit-article's direct outputs; hook outputs
        // (betaalopdracht, bezwaartermijn, motivering) arrive reactively.
        let besluit = service
            .evaluate_law(
                WPP_ID,
                &["subsidie_toegekend", "subsidiebedrag"],
                params.clone(),
                &date,
            )
            .map_err(|e| anyhow::anyhow!("subsidiebesluit berekenen mislukt: {e}"))?;

        let toegekend = as_bool(&besluit.outputs, "subsidie_toegekend");
        let bedrag = as_int(&besluit.outputs, "subsidiebedrag");
        let voldoet_transparantie = as_bool(&checks.outputs, "voldoet_aan_transparantie");
        let niveau = match params.get("niveau") {
            Some(Value::String(s)) => s.clone(),
            _ => "LANDELIJK".to_string(),
        };

        let motivering =
            build_motivering(&params, toegekend, bedrag, voldoet_transparantie, &niveau);

        Ok(BesluitUitkomst {
            subsidie_toegekend: toegekend,
            subsidiebedrag: bedrag,
            betaalopdracht_vereist: as_bool(&besluit.outputs, "betaalopdracht_vereist"),
            betaalopdracht_bedrag: as_int(&besluit.outputs, "betaalopdracht_bedrag"),
            bezwaartermijn_weken: as_int(&besluit.outputs, "bezwaartermijn_weken"),
            motivering_vereist: as_bool(&besluit.outputs, "motivering_vereist"),
            voldoet_aan_transparantie: voldoet_transparantie,
            heeft_recht_landelijk: as_bool(&checks.outputs, "heeft_recht_landelijk"),
            heeft_recht_decentraal: as_bool(&checks.outputs, "heeft_recht_decentraal"),
            outputs: serde_json::to_value(&besluit.outputs)?,
            motivering,
        })
    })
    .await?
}

/// Compute the bezwaartermijn dates after bekendmaking (AWB 6:8).
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
        let as_date = |name: &str| -> String {
            match result.outputs.get(name) {
                Some(Value::String(s)) => s.clone(),
                other => format!("{other:?}"),
            }
        };
        Ok(BezwaartermijnUitkomst {
            startdatum: as_date("bezwaartermijn_startdatum"),
            einddatum: as_date("bezwaartermijn_einddatum"),
        })
    })
    .await?
}

/// Convert JSON parameters (as stored/submitted) to engine values.
pub fn json_params_to_engine(
    params: &serde_json::Map<String, serde_json::Value>,
) -> BTreeMap<String, Value> {
    params
        .iter()
        .map(|(k, v)| (k.clone(), Value::from(v)))
        .collect()
}
