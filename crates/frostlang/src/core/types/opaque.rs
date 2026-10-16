//! [`FrostOpaque`]: the trait host data implements to travel through Frost.

use std::{any::Any, fmt::Debug, ops::Deref, sync::Arc};

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
/// out with [`OpaqueHandle::try_extract`].
pub trait FrostOpaque: Any + Debug + Send + Sync + sealed::DynTypeName {
    /// The host-facing name of this type of value, the same for every value.
    ///
    /// Surfaced when Frost renders the value and in errors; keep it short,
    /// stable, and capitalized like a type name. A `dyn FrostOpaque`, as an
    /// [`OpaqueHandle`] dereferences to, gives it through a `type_name` method
    /// of its own.
    fn type_name() -> &'static str
    where
        Self: Sized;

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
    /// be an equivalence relation, as [`Eq`] requires. In particular it must be
    /// reflexive: Frost relies on every value equaling itself.
    ///
    /// The default, `false`, makes `==` identity: a handle equals only itself
    /// and its clones. Which handles a script compares is decided by its source,
    /// never by compiler options: no optimization merges two handles into one
    /// or splits one into several. A type that implements [`Eq`] can defer to
    /// it:
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
    use super::FrostOpaque;

    /// Proof of a call from inside this crate; see
    /// [`FrostOpaque::has_drop_glue`](super::FrostOpaque::has_drop_glue).
    pub struct Token;

    /// [`FrostOpaque::type_name`] through a receiver, so that a
    /// `dyn FrostOpaque` can name its payload type. Implemented for every
    /// payload type, so no implementation can differ from its `type_name`.
    pub trait DynTypeName {
        fn dyn_type_name(&self) -> &'static str;
    }

    impl<T: FrostOpaque> DynTypeName for T {
        fn dyn_type_name(&self) -> &'static str {
            T::type_name()
        }
    }
}

impl dyn FrostOpaque {
    /// The payload type's [`FrostOpaque::type_name`].
    pub fn type_name(&self) -> &'static str {
        self.dyn_type_name()
    }

    /// [`std::mem::needs_drop`] for the payload's concrete type.
    pub(crate) fn needs_drop(&self) -> bool {
        self.has_drop_glue(sealed::Token)
    }

    /// Borrows the concrete `T`, or `None` if the payload is some other type.
    pub fn downcast_ref<T: FrostOpaque>(&self) -> Option<&T> {
        (self as &dyn Any).downcast_ref::<T>()
    }
}

/// A shared handle to host data in Frost: the payload of an `Opaque`
/// [`Value`](crate::Value).
///
/// It dereferences to the [`FrostOpaque`] payload, so the trait's methods apply
/// directly. Cloning a handle is cheap and shares the payload.
#[derive(Clone, Debug)]
pub struct OpaqueHandle(Arc<dyn FrostOpaque>);

impl OpaqueHandle {
    /// A handle to `payload`, the only one.
    pub fn new<T: FrostOpaque>(payload: T) -> Self {
        Self(Arc::new(payload))
    }

    /// Borrows the concrete `T`, or `None` if the payload is some other type.
    pub fn downcast_ref<T: FrostOpaque>(&self) -> Option<&T> {
        self.0.downcast_ref()
    }

    /// Mutably borrows the concrete `T`: `None` if the payload is some other
    /// type, or if another handle shares it.
    pub fn downcast_mut<T: FrostOpaque>(&mut self) -> Option<&mut T> {
        let payload: &mut dyn Any = Arc::get_mut(&mut self.0)?;
        payload.downcast_mut::<T>()
    }

    /// Takes the `T` out of this handle, zero-copy.
    ///
    /// Succeeds when the payload is a `T` and this is the only handle to it.
    /// Otherwise the handle comes back unchanged in `Err`, still usable: a
    /// shared handle or a different payload type is a normal outcome, not an
    /// error state.
    ///
    /// The owned counterpart of [`downcast_ref`](Self::downcast_ref), for a
    /// host that wants its data back without cloning.
    pub fn try_extract<T: FrostOpaque>(self) -> Result<T, Self> {
        Arc::try_unwrap(self.downcast_arc()?).map_err(Self::from_arc)
    }

    /// A handle to the payload of `payload`, shared with it.
    pub(crate) fn from_arc<T: FrostOpaque>(payload: Arc<T>) -> Self {
        Self(payload)
    }

    /// The payload as a `T`, still shared with any other handle to it, or
    /// this handle unchanged if the payload is some other type.
    pub(crate) fn downcast_arc<T: FrostOpaque>(self) -> Result<Arc<T>, Self> {
        if self.0.downcast_ref::<T>().is_none() {
            return Err(self);
        }
        let any: Arc<dyn Any + Send + Sync> = self.0;
        Ok(any
            .downcast()
            .expect("IMPOSSIBLE: payload type was checked above"))
    }

    /// Whether `self` and `other` are handles to the same payload.
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }

    /// Frost's `==` between two Opaques; see [`FrostOpaque::equals`].
    pub(crate) fn frost_eq(&self, other: &Self) -> bool {
        let same_type = (*self.0).type_id() == (*other.0).type_id();
        self.ptr_eq(other) || (same_type && self.0.equals(&*other.0))
    }
}

impl Deref for OpaqueHandle {
    type Target = dyn FrostOpaque;

    fn deref(&self) -> &(dyn FrostOpaque + 'static) {
        &*self.0
    }
}
