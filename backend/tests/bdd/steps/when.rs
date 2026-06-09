//! When steps: law execution.

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use crate::world::NappWorld;
use cucumber::when;

#[when("the subsidiebesluit is executed")]
fn execute_subsidiebesluit(world: &mut NappWorld) {
    world.execute_besluit();
}

#[when("the termijnverlenging is calculated")]
fn execute_termijnverlenging(world: &mut NappWorld) {
    world.execute("algemene_termijnenwet", &["verlengde_einddatum"]);
}

/// Orkestratie zoals de backend: AWB 4:13 (beslistermijn) gevolgd door de
/// Algemene termijnenwet op die einddatum.
#[when("the beslistermijn is calculated including the termijnenwet")]
fn execute_beslistermijn_keten(world: &mut NappWorld) {
    world.execute("algemene_wet_bestuursrecht", &["beslistermijn_einddatum"]);
    let einddatum = world
        .get_output("beslistermijn_einddatum")
        .cloned()
        .expect("AWB 4:13 leverde geen einddatum");
    world.parameters.insert("einddatum".to_string(), einddatum);
    world.execute("algemene_termijnenwet", &["verlengde_einddatum"]);
}

/// Orkestratie zoals de backend: eerst AWB 6:8 (einddatum), daarna de
/// Algemene termijnenwet (weekend-verlenging) op die einddatum.
#[when("the bezwaartermijn is calculated including the termijnenwet")]
fn execute_bezwaartermijn_keten(world: &mut NappWorld) {
    world.execute(
        "algemene_wet_bestuursrecht",
        &["bezwaartermijn_startdatum", "bezwaartermijn_einddatum"],
    );
    let einddatum = world
        .get_output("bezwaartermijn_einddatum")
        .cloned()
        .expect("AWB 6:8 leverde geen einddatum");
    world.parameters.insert("einddatum".to_string(), einddatum);
    world.execute("algemene_termijnenwet", &["verlengde_einddatum"]);
}
