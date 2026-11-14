//! `auth`: the headers that authenticate a request.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as Base64Engine;
use frostlang::{FrostResult, FrostString, NativeCtx, Value, frost_fn};

use crate::error;

/// An `authorization` header, as a Map of one header.
fn authorization(credentials: String) -> Value {
    Value::map([("authorization", Value::from(credentials))])
}

#[frost_fn]
pub(crate) fn bearer(token: FrostString) -> Value {
    authorization(format!("Bearer {}", token.as_str()))
}

#[frost_fn]
pub(crate) fn basic(
    #[frost(ctx)] ctx: &mut NativeCtx<'_>,
    user: FrostString,
    password: FrostString,
) -> FrostResult {
    // The colon separates the two, so a user cannot contain one; a password
    // can.
    if user.contains(':') {
        return Err(error::colon_in_user(ctx.name()));
    }
    let credentials = Base64Engine.encode(format!("{}:{}", user.as_str(), password.as_str()));
    Ok(authorization(format!("Basic {credentials}")))
}
