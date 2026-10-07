//! Building a `VmRuntimeConfiguration`: the default and each `with_*` method.
//! What each setting does to a run is covered by `vm_limits.rs`,
//! `vm_import.rs`, and `vm_print_sink.rs`.

use std::num::NonZeroUsize;
use std::sync::Arc;

use frostlang_runtime::{PrintSink, VmRuntimeConfiguration};

fn limit(n: usize) -> Option<NonZeroUsize> {
    NonZeroUsize::new(n)
}

/// The limits of `config`, in declaration order, for comparing them whole.
fn limits(config: &VmRuntimeConfiguration) -> [Option<NonZeroUsize>; 3] {
    [config.max_call_depth, config.fuel, config.max_import_depth]
}

#[test]
fn the_default_imposes_no_limits() {
    assert_eq!(limits(&VmRuntimeConfiguration::default()), [None; 3]);
}

#[test]
fn each_with_method_sets_only_its_own_limit() {
    let base = VmRuntimeConfiguration::default();
    assert_eq!(
        limits(&base.clone().with_max_call_depth(limit(1))),
        [limit(1), None, None],
        "with_max_call_depth"
    );
    assert_eq!(
        limits(&base.clone().with_fuel(limit(2))),
        [None, limit(2), None],
        "with_fuel"
    );
    assert_eq!(
        limits(&base.with_max_import_depth(limit(3))),
        [None, None, limit(3)],
        "with_max_import_depth"
    );
}

#[test]
fn a_limit_can_be_lifted_again() {
    let config = VmRuntimeConfiguration::default()
        .with_max_call_depth(limit(1))
        .with_fuel(limit(2))
        .with_max_import_depth(limit(3))
        .with_max_call_depth(None)
        .with_fuel(None)
        .with_max_import_depth(None);
    assert_eq!(limits(&config), [None; 3]);
}

#[test]
fn with_print_sink_sets_only_the_sink() {
    let sink: Arc<dyn PrintSink> = Arc::new(|_: &str| {});
    let config = VmRuntimeConfiguration::default()
        .with_fuel(limit(2))
        .with_print_sink(Arc::clone(&sink));
    assert!(Arc::ptr_eq(&config.print_sink, &sink));
    assert_eq!(
        limits(&config),
        [None, limit(2), None],
        "the limits are left as they were"
    );
}
