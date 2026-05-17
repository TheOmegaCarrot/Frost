use std::fmt;

#[derive(Clone, Debug)]
pub enum FrostError {
    Runtime(String),
    Type(String),
}

impl fmt::Display for FrostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrostError::Runtime(msg) => write!(f, "runtime error: {msg}"),
            FrostError::Type(msg) => write!(f, "type error: {msg}"),
        }
    }
}

impl std::error::Error for FrostError {}

use crate::Value;

pub type FrostResult = Result<Value, FrostError>;
