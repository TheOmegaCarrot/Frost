//! Taking a native's arguments as Rust types.

use std::fmt;
use std::ops::Range;

use enumset::{EnumSet, enum_set_union};
use serde::de::DeserializeOwned;

use crate::vm::{Binder, expected_types};
use crate::{
    FrostArray, FrostBytes, FrostError, FrostFloat, FrostMap, FrostString, FrostType, MapKey,
    Param, Params, Value, from_value,
};

/// A Rust type a native can take one argument as.
///
/// Implement it to take arguments as a type of your own. Each implementing type
/// is also a required parameter, through [`FrostArg`].
pub trait FromArg: Sized {
    /// The types of value this type is taken from.
    const TYPES: EnumSet<FrostType>;

    /// Converts `value` into this type.
    ///
    /// `value` has been type-checked: it is of one of [`TYPES`](Self::TYPES),
    /// and an implementation may rely on that, treating any other type as
    /// unreachable. Only its content remains to check, and content this type
    /// cannot take, as `-1` is for a `u64`, is rejected with an error from
    /// [`site.requires`](ArgSite::requires).
    fn from_arg(value: Value, site: &ArgSite<'_>) -> Result<Self, FrostError>;
}

/// A parameter of a native, as a Rust type: the [`Param`] it appears as, and
/// how [`Args::take`] takes its argument.
///
/// Every [`FromArg`] type is a required parameter, and [`Optional`] makes a
/// parameter that may be omitted. This trait is sealed: implement [`FromArg`]
/// instead.
pub trait FrostArg: sealed::TakeArg {
    /// The parameter, unnamed. Name it with [`Param::named`].
    const PARAM: Param;
}

mod sealed {
    use crate::FrostError;
    use crate::native::Args;

    /// How a [`FrostArg`](super::FrostArg) takes its argument, kept private so
    /// that the set of parameter kinds stays the crate's.
    pub trait TakeArg: Sized {
        fn take_from(args: &mut Args<'_>) -> Result<Self, FrostError>;
    }
}

impl<T: FromArg> FrostArg for T {
    const PARAM: Param = Param::of(T::TYPES);
}

impl<T: FromArg> sealed::TakeArg for T {
    fn take_from(args: &mut Args<'_>) -> Result<Self, FrostError> {
        let (site, value) = args.take_one();
        let value = value.unwrap_or_else(|| {
            panic!(
                "{site} of {} is required, but its arity let the call omit it",
                site.function()
            )
        });
        T::from_arg(value, &site)
    }
}

/// A parameter that may be omitted: `Optional(None)` when the call leaves it
/// out.
///
/// An explicit Null is an argument, not an omission. To accept Null, take a
/// [`Nullable`]; `Optional<Nullable<T>>` tells the two apart.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Optional<T>(pub Option<T>);

impl<T: FromArg> FrostArg for Optional<T> {
    const PARAM: Param = Param::of(T::TYPES).optional();
}

impl<T: FromArg> sealed::TakeArg for Optional<T> {
    fn take_from(args: &mut Args<'_>) -> Result<Self, FrostError> {
        let (site, value) = args.take_one();
        value
            .map(|value| T::from_arg(value, &site))
            .transpose()
            .map(Optional)
    }
}

/// A rest parameter: every argument after the others', possibly none, each
/// taken as `T`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Rest<T>(pub Vec<T>);

impl<T: FromArg> FrostArg for Rest<T> {
    const PARAM: Param = Param::of(T::TYPES).rest();
}

impl<T: FromArg> sealed::TakeArg for Rest<T> {
    fn take_from(args: &mut Args<'_>) -> Result<Self, FrostError> {
        args.take_rest()
            .map(|(site, value)| T::from_arg(value, &site))
            .collect::<Result<_, _>>()
            .map(Rest)
    }
}

/// An argument that may be Null as well as one of `T`'s types:
/// `Nullable(None)` when it is Null.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Nullable<T>(pub Option<T>);

impl<T: FromArg> FromArg for Nullable<T> {
    const TYPES: EnumSet<FrostType> = enum_set_union!(T::TYPES, FrostType::Null);

    fn from_arg(value: Value, site: &ArgSite<'_>) -> Result<Self, FrostError> {
        match value {
            Value::Null => Ok(Nullable(None)),
            value => T::from_arg(value, site).map(|taken| Nullable(Some(taken))),
        }
    }
}

/// An argument of any type, deserialized into `T` with [`from_value`], such
/// as a Map of options into a struct.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct De<T>(pub T);

impl<T: DeserializeOwned> FromArg for De<T> {
    const TYPES: EnumSet<FrostType> = FrostType::ANY;

    fn from_arg(value: Value, site: &ArgSite<'_>) -> Result<Self, FrostError> {
        from_value(value)
            .map(De)
            .map_err(|err| site.requires(format_args!("valid: {}", err.message())))
    }
}

/// The arguments of one call to a native, which [`take`](Self::take) takes in
/// order, each as a Rust type.
#[derive(Debug)]
pub struct Args<'a> {
    function: &'a str,
    params: Params,
    args: &'a mut [Value],
    /// How many parameters have been taken.
    taken: usize,
    binder: Binder,
}

impl<'a> Args<'a> {
    /// The arguments `args` of a call to the native named `function`, whose
    /// parameters are `params`.
    ///
    /// `args` must fit `params` in number and type, as they do in a native
    /// built with [`Value::checked_native`] from the same `params`.
    ///
    /// `params` is taken by value so that a `const` spec can be passed as is.
    /// Copying a spec built by [`Params::new`] is cheap; one built by
    /// [`Params::try_new`] copies its parameters.
    pub fn new(function: &'a str, params: Params, args: &'a mut [Value]) -> Self {
        let binder = Binder::new(&params, args.len());
        Self {
            function,
            params,
            args,
            taken: 0,
            binder,
        }
    }

    /// Takes the next parameter's argument as `T`, leaving Null in its place.
    ///
    /// Fails when the argument's content is unacceptable to `T`.
    ///
    /// # Panics
    ///
    /// When every parameter has been taken already, or when `T` cannot take
    /// the argument, which means the native's `Params` disagree with its
    /// argument types.
    pub fn take<T: FrostArg>(&mut self) -> Result<T, FrostError> {
        T::take_from(self)
    }

    /// The next parameter's name, and the indices of the arguments it takes.
    fn next_param(&mut self) -> (Option<&'static str>, Range<usize>) {
        let params = self.params.as_slice();
        let param = params.get(self.taken).unwrap_or_else(|| {
            panic!(
                "Function {} has {} parameters, all taken already",
                self.function,
                params.len()
            )
        });
        self.taken += 1;
        (param.name(), self.binder.bind(param))
    }

    /// The next parameter's site, and its argument, unless the call omits it.
    fn take_one(&mut self) -> (ArgSite<'a>, Option<Value>) {
        let (name, mut indices) = self.next_param();
        let site = ArgSite::argument(self.function, indices.start, name);
        let value = indices
            .next()
            .and_then(|index| self.args.get_mut(index))
            .map(Value::take);
        (site, value)
    }

    /// The next parameter's arguments, each with its site.
    fn take_rest(&mut self) -> impl Iterator<Item = (ArgSite<'a>, Value)> {
        let (name, indices) = self.next_param();
        let function = self.function;
        let start = indices.start;
        self.args[indices]
            .iter_mut()
            .enumerate()
            .map(move |(offset, value)| {
                (
                    ArgSite::argument(function, start + offset, name),
                    value.take(),
                )
            })
    }
}

/// Where a value is in a call to a native: one of its arguments, or an
/// element within one. It words the errors about that value.
///
/// It displays as the value's place, such as `argument 2 (args)` or
/// `element 3 of argument 2 (args)`.
#[derive(Clone, Copy, Debug)]
pub struct ArgSite<'a> {
    function: &'a str,
    place: Place<'a>,
}

#[derive(Clone, Copy, Debug)]
enum Place<'a> {
    /// An argument, by its index from 0, and its parameter's name, if any.
    Argument {
        index: usize,
        name: Option<&'static str>,
    },
    /// An element, by its index from 0, of the value at another site.
    Element { index: usize, of: &'a ArgSite<'a> },
}

impl<'a> ArgSite<'a> {
    /// The argument at `index`, from 0, of a call to the native named
    /// `function`, whose parameter there is named `name`.
    pub(crate) fn argument(function: &'a str, index: usize, name: Option<&'static str>) -> Self {
        Self {
            function,
            place: Place::Argument { index, name },
        }
    }

    /// The element at `index`, from 0, of the value at this site.
    pub fn element(&self, index: usize) -> ArgSite<'_> {
        ArgSite {
            function: self.function,
            place: Place::Element { index, of: self },
        }
    }

    /// The name of the native called.
    pub fn function(&self) -> &'a str {
        self.function
    }

    /// The error for a value at this site whose content is unacceptable.
    /// `requirement` completes `Function {name} requires {site} to be ...`, as
    /// `at least 0, got -1` does.
    pub fn requires(&self, requirement: impl fmt::Display) -> FrostError {
        FrostError::from_string(format!(
            "Function {} requires {self} to be {requirement}",
            self.function
        ))
    }

    /// The error for `value` at this site, which is not of `types`.
    pub(crate) fn wrong_type(&self, types: EnumSet<FrostType>, value: &Value) -> FrostError {
        FrostError::from_string(format!(
            "Function {} requires {} as {self}, got {}",
            self.function,
            expected_types(types),
            value.type_name()
        ))
    }
}

impl fmt::Display for ArgSite<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.place {
            Place::Argument {
                index,
                name: Some(name),
            } => write!(f, "argument {} ({name})", index + 1),
            Place::Argument { index, name: None } => write!(f, "argument {}", index + 1),
            Place::Element { index, of } => write!(f, "element {index} of {of}"),
        }
    }
}

/// Panics over `value`, which the type taking it at `site` cannot take: the
/// native's `Params` disagree with its argument types.
fn mismatch(site: &ArgSite<'_>, value: &Value) -> ! {
    panic!(
        "{site} of {} is {}, which its Rust type cannot take: \
         the native's Params disagree with its argument types",
        site.function(),
        value.type_name()
    )
}

// --- Implementations ---

impl FromArg for Value {
    const TYPES: EnumSet<FrostType> = FrostType::ANY;

    fn from_arg(value: Value, _: &ArgSite<'_>) -> Result<Self, FrostError> {
        Ok(value)
    }
}

/// Implements [`FromArg`] for each `$type`, taken from values of `$types` that
/// match `$pattern`, as `$taken`.
macro_rules! from_variant {
    ($($type:ty: $types:expr, $pattern:pat => $taken:expr;)*) => {$(
        impl FromArg for $type {
            const TYPES: EnumSet<FrostType> = $types;

            fn from_arg(value: Value, site: &ArgSite<'_>) -> Result<Self, FrostError> {
                match value {
                    $pattern => Ok($taken),
                    other => mismatch(site, &other),
                }
            }
        }
    )*};
}

from_variant! {
    bool: FrostType::BOOL, Value::Bool(b) => b;
    FrostFloat: FrostType::FLOAT, Value::Float(f) => f;
    String: FrostType::STRING, Value::String(s) => String::from(s);
    FrostString: FrostType::STRING, Value::String(s) => s;
    FrostBytes: FrostType::BYTES, Value::Bytes(b) => b;
    FrostArray: FrostType::ARRAY, Value::Array(a) => a;
    FrostMap: FrostType::MAP, Value::Map(m) => m;
}

impl FromArg for MapKey {
    const TYPES: EnumSet<FrostType> = MapKey::TYPES;

    fn from_arg(value: Value, site: &ArgSite<'_>) -> Result<Self, FrostError> {
        if !value.fits(MapKey::TYPES) {
            mismatch(site, &value)
        }
        Ok(MapKey::try_from(value).expect("a value of a key type is a key"))
    }
}

/// A Numeric argument: an Int becomes the nearest `f64`.
impl FromArg for f64 {
    const TYPES: EnumSet<FrostType> = FrostType::NUMERIC;

    fn from_arg(value: Value, site: &ArgSite<'_>) -> Result<Self, FrostError> {
        match value {
            Value::Int(i) => Ok(i as f64),
            Value::Float(f) => Ok(f.get()),
            other => mismatch(site, &other),
        }
    }
}

/// The Ints from `min` to `max`, the bounds of a Rust integer type, as an
/// [`ArgSite::requires`] requirement: as much of that range as an Int covers.
fn int_range(min: i128, max: i128) -> String {
    let (min, max) = (min.max(i64::MIN.into()), max.min(i64::MAX.into()));
    if max == i64::MAX.into() {
        format!("at least {min}")
    } else if min == i64::MIN.into() {
        format!("at most {max}")
    } else {
        format!("from {min} to {max}")
    }
}

/// Implements [`FromArg`] for each integer type, from an Int in its range.
macro_rules! from_int {
    ($($int:ty),*) => {$(
        impl FromArg for $int {
            const TYPES: EnumSet<FrostType> = FrostType::INT;

            fn from_arg(value: Value, site: &ArgSite<'_>) -> Result<Self, FrostError> {
                let Value::Int(int) = value else { mismatch(site, &value) };
                <$int>::try_from(int).map_err(|_| {
                    let range = int_range(
                        <$int>::MIN.try_into().unwrap_or(i128::MIN),
                        <$int>::MAX.try_into().unwrap_or(i128::MAX),
                    );
                    site.requires(format_args!("{range}, got {int}"))
                })
            }
        }
    )*};
}

from_int!(
    i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize
);

/// An Array, each element taken as `T`. An element not of `T`'s types is an
/// error worded as a wrong argument type is.
impl<T: FromArg> FromArg for Vec<T> {
    const TYPES: EnumSet<FrostType> = FrostType::ARRAY;

    fn from_arg(value: Value, site: &ArgSite<'_>) -> Result<Self, FrostError> {
        let Value::Array(array) = value else {
            mismatch(site, &value)
        };
        array
            .into_vec()
            .into_iter()
            .enumerate()
            .map(|(index, element)| {
                let site = site.element(index);
                if element.fits(T::TYPES) {
                    T::from_arg(element, &site)
                } else {
                    Err(site.wrong_type(T::TYPES, &element))
                }
            })
            .collect()
    }
}
