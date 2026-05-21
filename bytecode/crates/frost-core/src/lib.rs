//! Core value types and operations for the Frost language runtime.
//!
//! This crate defines [`Value`], the fundamental runtime type of Frost,
//! along with supporting types ([`FrostArray`], [`FrostMap`], [`FrostFloat`],
//! [`MapKey`]), error handling ([`FrostError`]), and serialization
//! ([`to_value`], [`from_value`]).

#![allow(unused)]

mod binary_operators;
mod error;
mod frost_array;
mod frost_float;
mod frost_map;
mod identifier_like;
mod serde_de;
mod serde_ser;
mod value;
mod value_compare;
mod value_impl;
mod value_to_string;

pub use error::{FrostError, FrostResult};
pub use frost_float::FrostFloat;
pub use serde_de::from_value;
pub use serde_ser::to_value;
pub use value::{Callable, FrostArray, FrostMap, MapKey, Value};
