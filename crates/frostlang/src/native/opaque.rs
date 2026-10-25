//! [`Opaque`]: host data of one type, as an argument or a result.

use std::fmt;
use std::ops::Deref;
use std::sync::Arc;

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::core::deserialize_of_types;
use crate::native::arg::mismatch;
use crate::native::{ArgSite, FromArg, IntoNativeResult};
use crate::{FrostError, FrostOpaque, FrostResult, FrostType, OpaqueHandle, TypeSet, Value};

/// An Opaque whose payload is a `T`, wherever values convert: as a native's
/// argument or result, and through [`from_value`](crate::from_value) and
/// [`to_value`](crate::to_value).
///
/// Its parameter accepts Opaque. An Opaque of another payload type fails,
/// naming both types by their [`FrostOpaque::type_name`]. Taking the argument
/// shares its payload rather than copying it, as does returning one.
///
/// ```
/// use frostlang::native::Opaque;
/// use frostlang::{FrostOpaque, frost_fn};
///
/// #[derive(Debug)]
/// struct Point(f64, f64);
///
/// impl FrostOpaque for Point {
///     fn type_name() -> &'static str {
///         "Point"
///     }
///
///     fn try_to_string(&self) -> Option<String> {
///         Some(format!("{}, {}", self.0, self.1))
///     }
/// }
///
/// #[frost_fn]
/// fn point(x: f64, y: f64) -> Opaque<Point> {
///     Opaque::new(Point(x, y))
/// }
///
/// #[frost_fn]
/// fn norm(point: Opaque<Point>) -> f64 {
///     point.0.hypot(point.1)
/// }
/// ```
pub struct Opaque<T>(Arc<T>);

impl<T: FrostOpaque> Opaque<T> {
    /// An Opaque holding `payload`.
    pub fn new(payload: T) -> Self {
        Self(Arc::new(payload))
    }

    /// The payload, taken out without copying it when this is its only
    /// holder; otherwise this Opaque unchanged.
    pub fn try_into_inner(self) -> Result<T, Self> {
        Arc::try_unwrap(self.0).map_err(Self)
    }
}

impl<T> Deref for Opaque<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

/// Shares the payload.
impl<T> Clone for Opaque<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<T: fmt::Debug> fmt::Debug for Opaque<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Opaque").field(&*self.0).finish()
    }
}

impl<T: FrostOpaque> FromArg for Opaque<T> {
    const TYPES: TypeSet = FrostType::OPAQUE;

    fn from_arg(value: Value, site: &ArgSite<'_>) -> Result<Self, FrostError> {
        let Value::Opaque(handle) = value else {
            mismatch(site, &value)
        };
        handle.downcast_arc().map(Self).map_err(|other| {
            site.requires(format_args!(
                "an Opaque {}, got an Opaque {}",
                T::type_name(),
                other.type_name()
            ))
        })
    }
}

impl<T: FrostOpaque> From<Opaque<T>> for Value {
    fn from(opaque: Opaque<T>) -> Self {
        Value::Opaque(OpaqueHandle::from_arc(opaque.0))
    }
}

impl<T: FrostOpaque> IntoNativeResult for Opaque<T> {
    fn into_result(self, _: &str) -> FrostResult {
        Ok(Value::from(self))
    }
}

/// Serializes as the [`Value`] it converts into.
impl<T: FrostOpaque> Serialize for Opaque<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        Value::from(self.clone()).serialize(serializer)
    }
}

/// Deserializes an Opaque whose payload is a `T`, failing on any other value.
impl<'de, T: FrostOpaque> Deserialize<'de> for Opaque<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let Value::Opaque(handle) = deserialize_of_types(deserializer, FrostType::OPAQUE)? else {
            unreachable!("a value of the Opaque type is an Opaque")
        };
        handle.downcast_arc().map(Self).map_err(|other| {
            D::Error::custom(format_args!(
                "expected an Opaque {}, got an Opaque {}",
                T::type_name(),
                other.type_name()
            ))
        })
    }
}
