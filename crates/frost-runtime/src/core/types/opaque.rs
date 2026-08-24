//! [`FrostOpaque`]: the trait host data implements to travel through Frost.

use std::{any::Any, borrow::Cow, fmt::Debug, sync::Arc};

use crate::core::Value;

/// Host data carried through Frost as an `Opaque` [`Value`].
///
/// Implementing this trait is all a host type needs to be handed into Frost:
/// wrap an instance with [`Value::opaque`] and it flows through scripts as an
/// inert value of Frost type `Opaque`. Frost code can store it and pass it
/// around, but never looks inside; equality is identity, and opaque values
/// refuse serialization. A native function receiving it back recovers the
/// concrete type with [`Value::downcast_opaque`], or steals it back out with
/// `try_extract`.
pub trait FrostOpaque: Any + Debug + Send + Sync {
    /// The host-facing name of this kind of value.
    ///
    /// Surfaced when Frost renders the value; keep it short, stable, and
    /// capitalized like a type name.
    fn type_name(&self) -> Cow<'static, str>;

    /// A human-readable approximation of this value, if it has one.
    ///
    /// `None` means nothing more useful than [`type_name`](Self::type_name)
    /// exists. The result is a rendering aid, never parsed back.
    fn try_to_string(&self) -> Option<String>;
}

impl dyn FrostOpaque {
    /// Borrows the concrete `T`, or `None` if the payload is some other type.
    pub fn downcast_ref<T: FrostOpaque>(&self) -> Option<&T> {
        (self as &dyn Any).downcast_ref::<T>()
    }

    /// Mutably borrows the concrete `T`, or `None` if the payload is some
    /// other type.
    ///
    /// Reaching `&mut dyn FrostOpaque` through the usual shared handle
    /// requires unique ownership; see [`Arc::get_mut`].
    pub fn downcast_mut<T: FrostOpaque>(&mut self) -> Option<&mut T> {
        (self as &mut dyn Any).downcast_mut::<T>()
    }

    /// Takes the `T` out of a uniquely-held handle, zero-copy.
    ///
    /// Succeeds when the payload is a `T` and this `Arc` is the only handle
    /// to it. Otherwise the handle comes back unchanged in `Err`, still
    /// usable: a shared handle or a different payload type is a normal
    /// outcome, not an error state.
    ///
    /// The owned counterpart of `downcast_ref`, for a host that wants its
    /// data back without cloning.
    pub fn try_extract<T: FrostOpaque>(self: Arc<Self>) -> Result<T, Arc<Self>> {
        if self.downcast_ref::<T>().is_none() {
            return Err(self);
        }

        let any: Arc<dyn Any + Send + Sync> = self;
        let typed: Arc<T> = any
            .downcast()
            .expect("IMPOSSIBLE: payload type was checked above");

        match Arc::try_unwrap(typed) {
            Ok(value) => Ok(value),
            Err(shared) => Err(shared),
        }
    }
}
