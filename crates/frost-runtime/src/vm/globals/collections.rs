//! Slicing, grouping, sorting, searching, and transforming arrays and maps.

use std::collections::BTreeMap;

use enumset::{EnumSet, enum_set};

use crate::{
    Arity, Bytecode, FrostArray, FrostError, FrostMap, FrostType, MapKey, NativeCtx, Param, Params,
    Value, ValueMap,
};

/// The types a Map key may have.
const MAP_KEY: EnumSet<FrostType> = enum_set!(
    FrostType::Bool | FrostType::Int | FrostType::Float | FrostType::String | FrostType::Bytes
);

const ONE_ARRAY: Params = Params::new(&[Param::of(FrostType::ARRAY)]);
const ONE_MAP: Params = Params::new(&[Param::of(FrostType::MAP)]);
const ARRAY_AND_FUNCTION: Params =
    Params::new(&[Param::of(FrostType::ARRAY), Param::of(FrostType::FUNCTION)]);
const MAP_AND_FUNCTION: Params =
    Params::new(&[Param::of(FrostType::MAP), Param::of(FrostType::FUNCTION)]);
const STRUCTURE_AND_FUNCTION: Params = Params::new(&[
    Param::of(FrostType::STRUCTURED),
    Param::of(FrostType::FUNCTION),
]);

/// The Array in a type-checked argument, taken from it.
fn take_array(arg: &mut Value) -> FrostArray {
    match arg.take() {
        Value::Array(array) => array,
        other => unreachable!("type-checked as an Array, got {}", other.type_name()),
    }
}

/// The Map in a type-checked argument, taken from it.
fn take_map(arg: &mut Value) -> FrostMap {
    match arg.take() {
        Value::Map(map) => map,
        other => unreachable!("type-checked as a Map, got {}", other.type_name()),
    }
}

/// The Int in a type-checked argument.
fn int_arg(arg: &Value) -> i64 {
    arg.as_int().expect("type-checked as an Int")
}

/// `n`, the argument at 1-based `position` of `function`, as a count of at least
/// `minimum`.
fn count_arg(function: &str, position: usize, n: i64, minimum: i64) -> Result<usize, FrostError> {
    if n < minimum {
        return Err(FrostError::from_string(format!(
            "Function {function} requires argument {position} to be at least {minimum}, got {n}"
        )));
    }
    Ok(usize::try_from(n).expect("a count at least its minimum is not negative"))
}

/// `value`, which `function`'s callback returned, as a Map key.
fn returned_key(function: &str, value: Value) -> Result<MapKey, FrostError> {
    if !value.fits(MAP_KEY) {
        return Err(FrostError::from_string(format!(
            "Function {function} requires its function to return a valid Map key, got {}",
            value.type_name()
        )));
    }
    Ok(MapKey::try_from(value).expect("the value fits a Map key"))
}

// --- Maps ---

pub(super) fn keys_global() -> Value {
    Value::checked_native("keys", ONE_MAP, |_, args| {
        let map = take_map(&mut args[0]);
        Ok(map.keys().cloned().map(Value::from).collect())
    })
}

pub(super) fn values_global() -> Value {
    Value::checked_native("values", ONE_MAP, |_, args| {
        let map = take_map(&mut args[0]);
        Ok(map.values().cloned().collect())
    })
}

pub(super) fn map_keys_global() -> Value {
    Value::checked_native("map_keys", MAP_AND_FUNCTION, |mut ctx, args| {
        let map = take_map(&mut args[0]).into_map();
        let function = args[1].take();
        let mut result = ValueMap::new();
        for (key, value) in map {
            let key = returned_key("map_keys", ctx.invoke(&function, [key.into()])?)?;
            result.insert(key, value);
        }
        Ok(result.into())
    })
}

pub(super) fn map_values_global() -> Value {
    Value::checked_native("map_values", MAP_AND_FUNCTION, |mut ctx, args| {
        let mut map = take_map(&mut args[0]).into_map();
        let function = args[1].take();
        for value in map.values_mut() {
            *value = ctx.invoke(&function, [value.take()])?;
        }
        Ok(map.into())
    })
}

pub(super) fn to_entries_global() -> Value {
    Value::checked_native("to_entries", ONE_MAP, |_, args| {
        let map = take_map(&mut args[0]);
        Ok(map
            .iter()
            .map(|(key, value)| {
                Value::map([("key", Value::from(key.clone())), ("value", value.clone())])
            })
            .collect())
    })
}

pub(super) fn from_entries_global() -> Value {
    Value::checked_native("from_entries", ONE_ARRAY, |_, args| {
        let entries = take_array(&mut args[0]);
        let mut result = ValueMap::new();
        for (i, entry) in entries.iter().enumerate() {
            let Value::Map(entry) = entry else {
                return Err(FrostError::from_string(format!(
                    "Function from_entries requires an Array of Maps, but element {i} is {}",
                    entry.type_name()
                )));
            };
            let field = |name: &str| {
                entry.get_str(name).ok_or_else(|| {
                    FrostError::from_string(format!(
                        "Function from_entries requires a {name} in every element, \
                         but element {i} has none"
                    ))
                })
            };
            let key = field("key")?;
            let value = field("value")?;
            if !key.fits(MAP_KEY) {
                return Err(FrostError::from_string(format!(
                    "Function from_entries requires a valid Map key in every element, \
                     but the key of element {i} is {}",
                    key.type_name()
                )));
            }
            let key = MapKey::try_from(key.clone()).expect("the key fits a Map key");
            result.insert(key, value.clone());
        }
        Ok(result.into())
    })
}

pub(super) fn dissoc_global() -> Value {
    Value::native("dissoc", Arity::AtLeast(1), |ctx, args| {
        ctx.check_args(args, &ONE_MAP)?;
        let key_param = Param::of(MAP_KEY);
        let mut map = take_map(&mut args[0]).into_map();
        for (i, key) in args.iter_mut().enumerate().skip(1) {
            if !key.fits(MAP_KEY) {
                return Err(FrostError::from_string(format!(
                    "Function dissoc requires {} as argument {}, got {}",
                    key_param.expected(),
                    i + 1,
                    key.type_name()
                )));
            }
            map.remove(&MapKey::try_from(key.take()).expect("the key fits a Map key"));
        }
        Ok(map.into())
    })
}

// --- Measuring and building ---

pub(super) fn len_global() -> Value {
    const MEASURABLE: EnumSet<FrostType> =
        enum_set!(FrostType::String | FrostType::Bytes | FrostType::Array | FrostType::Map);
    const PARAMS: Params = Params::new(&[Param::of(MEASURABLE)]);
    Value::checked_native("len", PARAMS, |_, args| {
        let len = match &args[0] {
            // A String's length is in code points; see the String/Bytes design.
            Value::String(text) => text.chars().count(),
            Value::Bytes(octets) => octets.len(),
            Value::Array(array) => array.len(),
            Value::Map(map) => map.len(),
            other => unreachable!("type-checked, got {}", other.type_name()),
        };
        Ok(Value::Int(
            i64::try_from(len).expect("a length fits in an Int"),
        ))
    })
}

pub(super) fn range_global() -> Value {
    const PARAMS: Params = Params::new(&[
        Param::of(FrostType::INT),
        Param::of(FrostType::INT).optional(),
        Param::of(FrostType::INT).optional(),
    ]);
    Value::checked_native("range", PARAMS, |_, args| {
        let (start, stop, step) = match *args {
            [ref stop] => (0, int_arg(stop), 1),
            [ref start, ref stop] => (int_arg(start), int_arg(stop), 1),
            [ref start, ref stop, ref step] => (int_arg(start), int_arg(stop), int_arg(step)),
            _ => unreachable!("arity-checked"),
        };
        if step == 0 {
            return Err(FrostError::from_static(
                "Function range requires a step other than 0",
            ));
        }
        let before_stop = |n: &i64| if step > 0 { *n < stop } else { *n > stop };
        // A step past the Int range ends the range, as it is past `stop` too.
        Ok(std::iter::successors(Some(start), |n| n.checked_add(step))
            .take_while(before_stop)
            .map(Value::Int)
            .collect())
    })
}

pub(super) fn nulls_global() -> Value {
    const PARAMS: Params = Params::new(&[Param::of(FrostType::INT)]);
    Value::checked_native("nulls", PARAMS, |_, args| {
        let count = count_arg("nulls", 1, int_arg(&args[0]), 0)?;
        Ok(vec![Value::Null; count].into())
    })
}

pub(super) fn repeat_global() -> Value {
    const PARAMS: Params = Params::new(&[Param::any(), Param::of(FrostType::INT)]);
    Value::checked_native("repeat", PARAMS, |_, args| {
        let count = count_arg("repeat", 2, int_arg(&args[1]), 0)?;
        Ok(vec![args[0].take(); count].into())
    })
}

pub(super) fn tile_global() -> Value {
    const SEQUENCE: EnumSet<FrostType> =
        enum_set!(FrostType::String | FrostType::Bytes | FrostType::Array);
    const PARAMS: Params = Params::new(&[Param::of(SEQUENCE), Param::of(FrostType::INT)]);
    Value::checked_native("tile", PARAMS, |_, args| {
        let count = count_arg("tile", 2, int_arg(&args[1]), 0)?;
        let size = match &args[0] {
            Value::String(text) => text.len(),
            Value::Bytes(octets) => octets.len(),
            Value::Array(array) => array.len() * size_of::<Value>(),
            other => unreachable!("type-checked, got {}", other.type_name()),
        };
        // Past `isize::MAX` bytes, allocating the result would panic.
        let fits = size
            .checked_mul(count)
            .is_some_and(|total| isize::try_from(total).is_ok());
        if !fits {
            return Err(FrostError::from_static(
                "Function tile cannot make a sequence that long",
            ));
        }
        Ok(match args[0].take() {
            Value::String(text) => text.repeat(count).into(),
            Value::Bytes(octets) => octets.repeat(count).into(),
            Value::Array(array) => std::iter::repeat_n(array.as_slice(), count)
                .flatten()
                .cloned()
                .collect(),
            other => unreachable!("type-checked, got {}", other.type_name()),
        })
    })
}

pub(super) fn id_global() -> Value {
    // Drop the closure's own value; the sole argument is already the answer.
    super::bytecode_global("id", Arity::Exact(1), vec![Bytecode::DropBelow(1)])
}

// --- Searching ---

pub(super) fn has_global() -> Value {
    const PARAMS: Params = Params::new(&[
        Param::of(FrostType::STRUCTURED),
        Param::of(MAP_KEY).named("index"),
    ]);
    Value::checked_native("has", PARAMS, |_, args| {
        Ok(Value::Bool(match (&args[0], &args[1]) {
            (Value::Array(array), Value::Int(i)) => array.frost_get(*i).is_some(),
            (Value::Array(_), index) => {
                return Err(FrostError::from_string(format!(
                    "Function has requires Int as argument 2 (index) for an Array, got {}",
                    index.type_name()
                )));
            }
            (Value::Map(map), key) => {
                map.contains_key(&MapKey::try_from(key.clone()).expect("type-checked as a key"))
            }
            (other, _) => unreachable!("type-checked, got {}", other.type_name()),
        }))
    })
}

pub(super) fn includes_global() -> Value {
    const PARAMS: Params = Params::new(&[Param::of(FrostType::ARRAY), Param::any()]);
    Value::checked_native("includes", PARAMS, |_, args| {
        let array = take_array(&mut args[0]);
        Ok(Value::Bool(array.iter().any(|element| *element == args[1])))
    })
}

pub(super) fn index_global() -> Value {
    super::generated::global("index")
}

pub(super) fn dig_global() -> Value {
    super::generated::global("dig")
}

/// Whether `function`'s answer for any element of `array` has the truthiness
/// `wanted`, asking in order and stopping at the first that does. Without a
/// function, each element's own truthiness is the answer.
fn any_answers(
    ctx: &mut NativeCtx<'_>,
    array: &FrostArray,
    function: Option<&Value>,
    wanted: bool,
) -> Result<bool, FrostError> {
    for element in array {
        let answer = match function {
            Some(function) => ctx.invoke_ref(function, [element])?.is_truthy(),
            None => element.is_truthy(),
        };
        if answer == wanted {
            return Ok(true);
        }
    }
    Ok(false)
}

const ARRAY_AND_OPTIONAL_FUNCTION: Params = Params::new(&[
    Param::of(FrostType::ARRAY),
    Param::of(FrostType::FUNCTION).optional(),
]);

pub(super) fn any_global() -> Value {
    Value::checked_native("any", ARRAY_AND_OPTIONAL_FUNCTION, |mut ctx, args| {
        let array = take_array(&mut args[0]);
        Ok(Value::Bool(any_answers(
            &mut ctx,
            &array,
            args.get(1),
            true,
        )?))
    })
}

pub(super) fn all_global() -> Value {
    Value::checked_native("all", ARRAY_AND_OPTIONAL_FUNCTION, |mut ctx, args| {
        let array = take_array(&mut args[0]);
        Ok(Value::Bool(!any_answers(
            &mut ctx,
            &array,
            args.get(1),
            false,
        )?))
    })
}

pub(super) fn none_global() -> Value {
    Value::checked_native("none", ARRAY_AND_OPTIONAL_FUNCTION, |mut ctx, args| {
        let array = take_array(&mut args[0]);
        Ok(Value::Bool(!any_answers(
            &mut ctx,
            &array,
            args.get(1),
            true,
        )?))
    })
}

pub(super) fn find_global() -> Value {
    const PARAMS: Params = Params::new(&[
        Param::of(FrostType::ARRAY),
        Param::of(FrostType::FUNCTION).named("predicate"),
    ]);
    Value::checked_native("find", PARAMS, |mut ctx, args| {
        let array = take_array(&mut args[0]);
        for element in array.into_vec() {
            if ctx.invoke_ref(&args[1], [&element])?.is_truthy() {
                return Ok(element);
            }
        }
        Ok(Value::Null)
    })
}

// --- Slicing ---

pub(super) fn slice_global() -> Value {
    super::stub("slice")
}

pub(super) fn stride_global() -> Value {
    super::stub("stride")
}

pub(super) fn take_global() -> Value {
    super::stub("take")
}

pub(super) fn drop_global() -> Value {
    super::stub("drop")
}

pub(super) fn tail_global() -> Value {
    super::stub("tail")
}

pub(super) fn drop_tail_global() -> Value {
    super::stub("drop_tail")
}

const ARRAY_AND_SIZE: Params =
    Params::new(&[Param::of(FrostType::ARRAY), Param::of(FrostType::INT)]);

pub(super) fn slide_global() -> Value {
    Value::checked_native("slide", ARRAY_AND_SIZE, |_, args| {
        let size = count_arg("slide", 2, int_arg(&args[1]), 1)?;
        let array = take_array(&mut args[0]);
        Ok(array.as_slice().windows(size).map(Value::from).collect())
    })
}

pub(super) fn chunk_global() -> Value {
    Value::checked_native("chunk", ARRAY_AND_SIZE, |_, args| {
        let size = count_arg("chunk", 2, int_arg(&args[1]), 1)?;
        let array = take_array(&mut args[0]);
        Ok(array.as_slice().chunks(size).map(Value::from).collect())
    })
}

pub(super) fn reverse_global() -> Value {
    const SEQUENCE: EnumSet<FrostType> =
        enum_set!(FrostType::String | FrostType::Bytes | FrostType::Array);
    const PARAMS: Params = Params::new(&[Param::of(SEQUENCE)]);
    Value::checked_native("reverse", PARAMS, |_, args| {
        Ok(match args[0].take() {
            // A String reverses by code point; see the String/Bytes design.
            Value::String(text) => text.chars().rev().collect::<String>().into(),
            Value::Bytes(octets) => octets.iter().rev().copied().collect::<Vec<u8>>().into(),
            Value::Array(array) => {
                let mut elements = array.into_vec();
                elements.reverse();
                elements.into()
            }
            other => unreachable!("type-checked, got {}", other.type_name()),
        })
    })
}

/// How many leading elements of `array` `predicate` holds for, asking each once
/// and stopping at the first it does not.
fn leading_matches(
    ctx: &mut NativeCtx<'_>,
    array: &[Value],
    predicate: &Value,
) -> Result<usize, FrostError> {
    for (i, element) in array.iter().enumerate() {
        if !ctx.invoke_ref(predicate, [element])?.is_truthy() {
            return Ok(i);
        }
    }
    Ok(array.len())
}

pub(super) fn take_while_global() -> Value {
    Value::checked_native("take_while", ARRAY_AND_FUNCTION, |mut ctx, args| {
        let mut elements = take_array(&mut args[0]).into_vec();
        let kept = leading_matches(&mut ctx, &elements, &args[1])?;
        elements.truncate(kept);
        Ok(elements.into())
    })
}

pub(super) fn drop_while_global() -> Value {
    Value::checked_native("drop_while", ARRAY_AND_FUNCTION, |mut ctx, args| {
        let mut elements = take_array(&mut args[0]).into_vec();
        let dropped = leading_matches(&mut ctx, &elements, &args[1])?;
        elements.drain(..dropped);
        Ok(elements.into())
    })
}

pub(super) fn chunk_by_global() -> Value {
    super::stub("chunk_by")
}

/// Append `elements` to `out`, splicing in the elements of each Array among them
/// to `depth` levels, or to every level when `depth` is `None`.
fn flatten_into(out: &mut Vec<Value>, elements: Vec<Value>, depth: Option<usize>) {
    for element in elements {
        match element {
            Value::Array(inner) if depth != Some(0) => {
                flatten_into(out, inner.into_vec(), depth.map(|depth| depth - 1));
            }
            other => out.push(other),
        }
    }
}

pub(super) fn flatten_global() -> Value {
    const PARAMS: Params = Params::new(&[
        Param::of(FrostType::ARRAY),
        Param::of(FrostType::INT).optional(),
    ]);
    Value::checked_native("flatten", PARAMS, |_, args| {
        let depth = match args.get(1) {
            Some(depth) => Some(count_arg("flatten", 2, int_arg(depth), 0)?),
            None => None,
        };
        let mut out = Vec::new();
        flatten_into(&mut out, take_array(&mut args[0]).into_vec(), depth);
        Ok(out.into())
    })
}

pub(super) fn zip_global() -> Value {
    super::stub("zip")
}

pub(super) fn zip_with_global() -> Value {
    super::stub("zip_with")
}

pub(super) fn xprod_global() -> Value {
    super::stub("xprod")
}

pub(super) fn xprod_with_global() -> Value {
    super::stub("xprod_with")
}

// --- Transforming ---

pub(super) fn transform_global() -> Value {
    Value::checked_native("transform", STRUCTURE_AND_FUNCTION, |mut ctx, args| {
        let structure = args[0].take();
        let function = args[1].take();

        match structure {
            Value::Array(arr) => {
                let mut vec = arr.into_vec();
                for elem in &mut vec {
                    *elem = ctx.invoke(&function, [elem.take()])?;
                }
                Ok(vec.into())
            }
            Value::Map(map) => {
                let map = map.into_map();
                let mut result = ValueMap::new();
                for (k, v) in map {
                    match ctx.invoke(&function, [k.into(), v])? {
                        Value::Map(m) => result.extend(m.into_map()),
                        other => {
                            return Err(FrostError::from_string(format!(
                                "When transforming a Map, the function must return a Map, got {}",
                                other.type_name()
                            )));
                        }
                    }
                }
                Ok(result.into())
            }
            _ => unreachable!(),
        }
    })
}

pub(super) fn flat_map_global() -> Value {
    Value::checked_native("flat_map", ARRAY_AND_FUNCTION, |mut ctx, args| {
        let array = take_array(&mut args[0]);
        let mut out = Vec::new();
        for element in array.into_vec() {
            match ctx.invoke(&args[1], [element])? {
                Value::Array(inner) => out.extend(inner.into_vec()),
                other => out.push(other),
            }
        }
        Ok(out.into())
    })
}

/// The elements of an Array, or the entries of a Map, whose `predicate` answer
/// has the truthiness `keep`: the body of both `select` and `reject`.
fn filter_structure(
    ctx: &mut NativeCtx<'_>,
    structure: Value,
    predicate: &Value,
    keep: bool,
) -> Result<Value, FrostError> {
    match structure {
        Value::Array(arr) => arr
            .into_vec()
            .into_iter()
            .filter_map(|elem| match ctx.invoke_ref(predicate, [&elem]) {
                Ok(res) if res.is_truthy() == keep => Some(Ok(elem)),
                Ok(_) => None,
                Err(e) => Some(Err(e)),
            })
            .collect(),
        Value::Map(map) => map
            .into_map()
            .into_iter()
            .filter_map(
                |(k, v)| match ctx.invoke(predicate, [k.clone().into(), v.clone()]) {
                    Ok(res) if res.is_truthy() == keep => Some(Ok((k, v))),
                    Ok(_) => None,
                    Err(e) => Some(Err(e)),
                },
            )
            .collect(),
        other => unreachable!("type-checked as Structured, got {}", other.type_name()),
    }
}

pub(super) fn select_global() -> Value {
    Value::checked_native("select", STRUCTURE_AND_FUNCTION, |mut ctx, args| {
        filter_structure(&mut ctx, args[0].take(), &args[1], true)
    })
}

pub(super) fn reject_global() -> Value {
    Value::checked_native("reject", STRUCTURE_AND_FUNCTION, |mut ctx, args| {
        filter_structure(&mut ctx, args[0].take(), &args[1], false)
    })
}

pub(super) fn fold_global() -> Value {
    const PARAMS: Params = Params::new(&[
        Param::of(FrostType::STRUCTURED),
        Param::of(FrostType::FUNCTION),
        Param::any().optional(),
    ]);
    Value::checked_native("fold", PARAMS, |mut ctx, args| {
        let structure = args[0].take();
        let function = args[1].take();

        match structure {
            Value::Array(arr) => {
                let mut iter = arr.into_vec().into_iter();
                let init = match args.get_mut(2).map(Value::take) {
                    Some(init) => init,
                    None => iter.next().unwrap_or(Value::Null),
                };

                iter.try_fold(init, |acc, elem| ctx.invoke(&function, [acc, elem]))
            }
            Value::Map(map) => {
                let Some(init) = args.get_mut(2).map(Value::take) else {
                    return Err(FrostError::from_static(
                        "Fold over a Map requires an initializer",
                    ));
                };
                map.into_map().into_iter().try_fold(init, |acc, (k, v)| {
                    ctx.invoke(&function, [acc, k.into(), v])
                })
            }
            _ => unreachable!(),
        }
    })
}

pub(super) fn sum_global() -> Value {
    Value::checked_native("sum", ONE_ARRAY, |_, args| {
        let mut elements = take_array(&mut args[0]).into_vec().into_iter();
        let Some(first) = elements.next() else {
            return Ok(Value::Null);
        };
        elements.try_fold(first, Value::add_owned)
    })
}

pub(super) fn product_global() -> Value {
    Value::checked_native("product", ONE_ARRAY, |_, args| {
        let mut elements = take_array(&mut args[0]).into_vec().into_iter();
        let Some(first) = elements.next() else {
            return Ok(Value::Null);
        };
        elements.try_fold(first, |product, element| product.multiply(&element))
    })
}

pub(super) fn sorted_global() -> Value {
    super::stub("sorted")
}

pub(super) fn sort_by_global() -> Value {
    super::stub("sort_by")
}

// --- Grouping ---

pub(super) fn group_by_global() -> Value {
    Value::checked_native("group_by", ARRAY_AND_FUNCTION, |mut ctx, args| {
        let array = take_array(&mut args[0]);
        let mut groups: BTreeMap<MapKey, Vec<Value>> = BTreeMap::new();
        for element in array.into_vec() {
            let key = returned_key("group_by", ctx.invoke_ref(&args[1], [&element])?)?;
            groups.entry(key).or_default().push(element);
        }
        Ok(groups
            .into_iter()
            .map(|(key, group)| (key, Value::from(group)))
            .collect())
    })
}

pub(super) fn count_by_global() -> Value {
    Value::checked_native("count_by", ARRAY_AND_FUNCTION, |mut ctx, args| {
        let array = take_array(&mut args[0]);
        let mut counts: BTreeMap<MapKey, i64> = BTreeMap::new();
        for element in &array {
            let key = returned_key("count_by", ctx.invoke_ref(&args[1], [element])?)?;
            *counts.entry(key).or_default() += 1;
        }
        Ok(counts
            .into_iter()
            .map(|(key, count)| (key, Value::Int(count)))
            .collect())
    })
}

pub(super) fn scan_global() -> Value {
    Value::checked_native("scan", ARRAY_AND_FUNCTION, |mut ctx, args| {
        let mut elements = take_array(&mut args[0]).into_vec().into_iter();
        let Some(first) = elements.next() else {
            return Ok(Value::from(Vec::<Value>::new()));
        };
        let mut running = vec![first];
        for element in elements {
            let last = running.last().expect("running starts with an element");
            let next = ctx.invoke(&args[1], [last.clone(), element])?;
            running.push(next);
        }
        Ok(running.into())
    })
}

pub(super) fn partition_global() -> Value {
    Value::checked_native("partition", ARRAY_AND_FUNCTION, |mut ctx, args| {
        let array = take_array(&mut args[0]);
        let (mut pass, mut fail) = (Vec::new(), Vec::new());
        for element in array.into_vec() {
            if ctx.invoke_ref(&args[1], [&element])?.is_truthy() {
                pass.push(element);
            } else {
                fail.push(element);
            }
        }
        Ok(Value::map([("pass", pass.into()), ("fail", fail.into())]))
    })
}

pub(super) fn map_into_global() -> Value {
    Value::checked_native("map_into", ARRAY_AND_FUNCTION, |mut ctx, args| {
        let array = take_array(&mut args[0]);
        let mut result = ValueMap::new();
        for element in array.into_vec() {
            match ctx.invoke(&args[1], [element])? {
                Value::Map(entries) => result.extend(entries.into_map()),
                other => {
                    return Err(FrostError::from_string(format!(
                        "Function map_into requires its function to return a Map, got {}",
                        other.type_name()
                    )));
                }
            }
        }
        Ok(result.into())
    })
}

pub(super) fn each_global() -> Value {
    Value::checked_native("each", STRUCTURE_AND_FUNCTION, |mut ctx, params| {
        let structure = params[0].take();
        let function = params[1].take();
        match &structure {
            Value::Array(arr) => {
                for v in arr {
                    ctx.invoke_ref(&function, [v])?;
                }
            }
            Value::Map(map) => {
                for (k, v) in map {
                    ctx.invoke(&function, [k.clone().into(), v.clone()])?;
                }
            }
            _ => unreachable!(),
        };

        Ok(structure)
    })
}
