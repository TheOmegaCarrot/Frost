//! The loads in progress on each thread, for detecting import cycles.

use std::cell::RefCell;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use frostlang_runtime::FrostError;

/// Tells one resolver's loads from another's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ResolverId(u64);

impl ResolverId {
    /// An id no other resolver has.
    pub(crate) fn unique() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        Self(NEXT.fetch_add(1, Ordering::Relaxed))
    }
}

#[derive(Debug)]
struct Load {
    resolver: ResolverId,
    spec: String,
    path: PathBuf,
}

thread_local! {
    // Innermost last. A module runs on the thread that imports it, so a module
    // that imports itself, however indirectly, finds its own load here.
    static LOADS: RefCell<Vec<Load>> = const { RefCell::new(Vec::new()) };
}

/// A load in progress on this thread, from [`enter`](Self::enter) until it is
/// dropped, by a panic included.
#[derive(Debug)]
pub(crate) struct Loading {
    // Recorded on this thread, so it must end on this thread.
    _not_send: PhantomData<*const ()>,
}

impl Loading {
    /// Begin `resolver`'s load of the module at `path`, imported as `spec`,
    /// unless that module is already loading on this thread: then this import
    /// closes a cycle, an error naming its chain of imports.
    pub(crate) fn enter(resolver: ResolverId, spec: &str, path: &Path) -> Result<Self, FrostError> {
        LOADS.with_borrow_mut(|loads| {
            let own = loads.iter().filter(|load| load.resolver == resolver);
            if let Some(start) = own.clone().position(|load| load.path == path) {
                let chain: Vec<&str> = own
                    .skip(start)
                    .map(|load| load.spec.as_str())
                    .chain([spec])
                    .collect();
                return Err(FrostError::from_string(format!(
                    "Import cycle: {}",
                    chain.join(" -> ")
                )));
            }
            loads.push(Load {
                resolver,
                spec: spec.to_owned(),
                path: path.to_owned(),
            });
            Ok(Self {
                _not_send: PhantomData,
            })
        })
    }
}

impl Drop for Loading {
    fn drop(&mut self) {
        // Loads nest strictly on a thread, so the innermost is this one.
        LOADS.with_borrow_mut(|loads| {
            loads.pop();
        });
    }
}
