#![allow(unused)]

mod error;
mod frost_float;
mod value;
mod value_impl;

pub use error::{FrostError, FrostResult};
pub use frost_float::FrostFloat;
pub use value::{Callable, FrostArray, FrostMap, MapKey, Value};
