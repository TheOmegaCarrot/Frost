//! `frost`: the reference Frost command-line interface.

use frost_driver::{Driver, Exit};
use frost_runtime::{ImporterBuilder, Stdlib, stdlib::RandomConfig};

fn main() -> Exit {
    Driver::new()
        .with_name("frost")
        .with_version(env!("CARGO_PKG_VERSION"))
        .with_importer(
            ImporterBuilder::new()
                .with_stdlib(Stdlib::complete(RandomConfig::default()))
                .build(),
        )
        .run_from_env()
}
