//! `frost`: the reference Frost command-line interface.

use frostlang_driver::{Driver, Exit};
use frostlang_runtime::{ImporterBuilder, Stdlib, stdlib::RandomConfig};

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
