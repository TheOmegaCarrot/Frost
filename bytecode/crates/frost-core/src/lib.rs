//! Core value types and operations for the Frost language runtime.
//!
//! This crate defines [`Value`], the fundamental runtime type of Frost,
//! along with supporting types ([`FrostArray`], [`FrostMap`], [`FrostFloat`],
//! [`MapKey`]), error handling ([`FrostError`]), and serialization
//! ([`to_value`], [`from_value`]).

#![allow(unused)]

mod error;
mod serde;
mod types;
pub(crate) mod util;
mod value;

pub use error::{FrostError, FrostResult};
pub use serde::{from_value, to_value};
pub use types::float::FrostFloat;
pub use value::{Callable, FrostArray, FrostMap, MapKey, Value};
