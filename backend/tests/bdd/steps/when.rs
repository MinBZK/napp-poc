//! When steps: law execution.

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use crate::world::NappWorld;
use cucumber::when;

#[when("the subsidiebesluit is executed")]
fn execute_subsidiebesluit(world: &mut NappWorld) {
    world.execute_besluit();
}
