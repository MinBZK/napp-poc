//! BDD test runner for the NAPP subsidy law.
//!
//! Runs the Gherkin scenarios in `scenarios/` against the regelrecht engine
//! with the laws from `law/` loaded.
//!
//! ```bash
//! cargo test --test bdd -- --nocapture
//! # or: just bdd
//! ```

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

mod steps;
mod world;

use cucumber::World;
use std::path::Path;

#[tokio::main]
async fn main() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let scenarios_dir = Path::new(manifest_dir)
        .parent() // project root
        .map(|p| p.join("scenarios"))
        .expect("Could not find scenarios directory");

    if !scenarios_dir.exists() {
        panic!("Scenarios directory not found: {}", scenarios_dir.display());
    }

    world::NappWorld::cucumber()
        .max_concurrent_scenarios(1)
        .with_default_cli()
        .run_and_exit(scenarios_dir)
        .await;
}
