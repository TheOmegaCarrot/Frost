//! [`FrostOpaque`]: the trait host data implements to travel through Frost.

use std::{any::Any, borrow::Cow, fmt::Debug, sync::Arc};

/// Host data carried through Frost as an `Opaque` [`Value`](crate::Value).
///
/// Implementing this trait is all a host type needs to be handed into Frost:
/// wrap an instance with [`Value::opaque`](crate::Value::opaque) and it flows through scripts as an
/// inert value of Frost type `Opaque`. Frost code can store it, pass it
/// around, and compare it with `==` (see [`equals`](Self::equals)), but never
/// looks inside. Opaque values refuse serialization, except a
/// [`SpecialFloat`](crate::SpecialFloat). A native function receiving it back
/// recovers the concrete type with
/// [`Value::downcast_opaque`](crate::Value::downcast_opaque), or steals it back
/// out with `try_extract`.
pub trait FrostOpaque: Any + Debug + Send + Sync {
    /// The host-facing name of this kind of value.
    ///
    /// Surfaced when Frost renders the value; keep it short, stable, and
    /// capitalized like a type name.
    fn type_name(&self) -> Cow<'static, str>;

    /// A human-readable approximation of this value, if it has one.
    ///
    /// Frost renders the value as `<TypeName: approximation>`, or as
    /// `<TypeName>` when this is `None`. The result is a rendering aid, never
    /// parsed back.
    fn try_to_string(&self) -> Option<String>;

    /// Whether this equals `other`, for Frost's `==`.
    ///
    /// Frost calls this only for two distinct handles to payloads of the same
    /// concrete type, so `other` always downcasts to `Self`; a handle always
    /// equals itself, and payloads of different types are never equal. It must
    /// be an equivalence relation, as [`Eq`] requires.
    ///
    /// The default, `false`, makes `==` identity. A type that implements
    /// [`Eq`] can defer to it:
    ///
    /// ```ignore
    /// fn equals(&self, other: &dyn FrostOpaque) -> bool {
    ///     other.downcast_ref::<Self>().is_some_and(|other| self == other)
    /// }
    /// ```
    fn equals(&self, other: &dyn FrostOpaque) -> bool {
        let _ = other;
        false
    }

    /// Whether this payload's type has drop glue.
    ///
    /// The token type is unnameable outside this crate, so no implementation
    /// can override this: its answer is the compiler's, per concrete type.
    #[doc(hidden)]
    fn has_drop_glue(&self, _: sealed::Token) -> bool {
        std::mem::needs_drop::<Self>()
    }
}

pub(crate) mod sealed {
    /// Proof of a call from inside this crate; see
    /// [`FrostOpaque::has_drop_glue`](super::FrostOpaque::has_drop_glue).
    pub struct Token;
}

impl dyn FrostOpaque {
    /// Frost's `==` between two Opaques; see [`FrostOpaque::equals`].
    pub(crate) fn frost_eq(self: &Arc<Self>, other: &Arc<Self>) -> bool {
        let same_type = (**self).type_id() == (**other).type_id();
        Arc::ptr_eq(self, other) || (same_type && self.equals(&**other))
    }

    /// [`std::mem::needs_drop`] for the payload's concrete type.
    pub(crate) fn needs_drop(&self) -> bool {
        self.has_drop_glue(sealed::Token)
    }

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
