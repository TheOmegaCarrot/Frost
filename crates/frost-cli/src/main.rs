//! `frost`: the reference Frost command-line interface.

use frost_driver::{Driver, Exit};

fn main() -> Exit {
    Driver::new()
        .with_name("frost")
        .with_version(env!("CARGO_PKG_VERSION"))
        .run_from_env()
}
