#![allow(unused)]

mod binary_operators;
mod error;
mod frost_array;
mod frost_float;
mod frost_map;
mod identifier_like;
mod value;
mod value_compare;
mod value_impl;
mod value_to_string;

pub use error::{FrostError, FrostResult};
pub use frost_float::FrostFloat;
pub use value::{Callable, FrostArray, FrostMap, MapKey, Value};
