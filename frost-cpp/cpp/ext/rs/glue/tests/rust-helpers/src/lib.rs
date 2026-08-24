use frost_glue::{FrostFunction, FrostRef, FrostValue};

#[cxx::bridge(namespace = "frst::rs::test")]
mod ffi {
    unsafe extern "C++" {
        include!("frost-glue.hpp");
        #[namespace = "frst::rs"]
        type Value = frost_glue::ffi::Value;
    }

    extern "Rust" {
        // ---- Basic value passing ----
        fn rt_identity(args: &[SharedPtr<Value>]) -> Result<SharedPtr<Value>>;
        fn rt_type_name(args: &[SharedPtr<Value>]) -> Result<SharedPtr<Value>>;
        fn rt_round_trip(args: &[SharedPtr<Value>]) -> Result<SharedPtr<Value>>;

        // ---- Array operations ----
        fn rt_array_sum(args: &[SharedPtr<Value>]) -> Result<SharedPtr<Value>>;
        fn rt_make_array(args: &[SharedPtr<Value>]) -> Result<SharedPtr<Value>>;
        fn rt_array_reverse(args: &[SharedPtr<Value>]) -> Result<SharedPtr<Value>>;

        // ---- Map operations ----
        fn rt_make_map(args: &[SharedPtr<Value>]) -> Result<SharedPtr<Value>>;
        fn rt_map_keys_sorted(args: &[SharedPtr<Value>]) -> Result<SharedPtr<Value>>;
        fn rt_map_get_by_key(args: &[SharedPtr<Value>]) -> Result<SharedPtr<Value>>;
        fn rt_map_entry_count(args: &[SharedPtr<Value>]) -> Result<SharedPtr<Value>>;

        // ---- String / binary operations ----
        fn rt_concat_all(args: &[SharedPtr<Value>]) -> Result<SharedPtr<Value>>;
        fn rt_string_byte_len(args: &[SharedPtr<Value>]) -> Result<SharedPtr<Value>>;

        // ---- Unpack (FrostRef enum) ----
        fn rt_describe_type(args: &[SharedPtr<Value>]) -> Result<SharedPtr<Value>>;

        // ---- Function operations ----
        fn rt_call_callback(args: &[SharedPtr<Value>]) -> Result<SharedPtr<Value>>;
        fn rt_make_adder(args: &[SharedPtr<Value>]) -> Result<SharedPtr<Value>>;
        fn rt_make_failing_fn(args: &[SharedPtr<Value>]) -> Result<SharedPtr<Value>>;

        // ---- Map null-value fidelity ----
        fn rt_map_lookup_found(args: &[SharedPtr<Value>]) -> Result<SharedPtr<Value>>;

        // ---- FrostArray::iter() ----
        fn rt_array_iter_strings(args: &[SharedPtr<Value>]) -> Result<SharedPtr<Value>>;

        // ---- Category checks ----
        fn rt_is_numeric(args: &[SharedPtr<Value>]) -> Result<SharedPtr<Value>>;
        fn rt_is_primitive(args: &[SharedPtr<Value>]) -> Result<SharedPtr<Value>>;
        fn rt_is_structured(args: &[SharedPtr<Value>]) -> Result<SharedPtr<Value>>;

        // ---- serde deserialization ----
        fn rt_serde_flat_struct(args: &[SharedPtr<Value>]) -> Result<SharedPtr<Value>>;
        fn rt_serde_optional_field(args: &[SharedPtr<Value>]) -> Result<SharedPtr<Value>>;
        fn rt_serde_nested(args: &[SharedPtr<Value>]) -> Result<SharedPtr<Value>>;
        fn rt_serde_vec(args: &[SharedPtr<Value>]) -> Result<SharedPtr<Value>>;
        fn rt_serde_primitives(args: &[SharedPtr<Value>]) -> Result<SharedPtr<Value>>;

        // ---- require_args validation ----
        fn rt_require_nullary(args: &[SharedPtr<Value>]) -> Result<SharedPtr<Value>>;
        fn rt_require_one_int(args: &[SharedPtr<Value>]) -> Result<SharedPtr<Value>>;
        fn rt_require_string_opt_int(args: &[SharedPtr<Value>]) -> Result<SharedPtr<Value>>;
        fn rt_require_any(args: &[SharedPtr<Value>]) -> Result<SharedPtr<Value>>;

        // ---- Arithmetic / comparison / coercion ----
        fn rt_add(args: &[SharedPtr<Value>]) -> Result<SharedPtr<Value>>;
        fn rt_subtract(args: &[SharedPtr<Value>]) -> Result<SharedPtr<Value>>;
        fn rt_multiply(args: &[SharedPtr<Value>]) -> Result<SharedPtr<Value>>;
        fn rt_divide(args: &[SharedPtr<Value>]) -> Result<SharedPtr<Value>>;
        fn rt_modulus(args: &[SharedPtr<Value>]) -> Result<SharedPtr<Value>>;
        fn rt_equal(args: &[SharedPtr<Value>]) -> Result<SharedPtr<Value>>;
        fn rt_not_equal(args: &[SharedPtr<Value>]) -> Result<SharedPtr<Value>>;
        fn rt_less_than(args: &[SharedPtr<Value>]) -> Result<SharedPtr<Value>>;
        fn rt_less_than_or_equal(args: &[SharedPtr<Value>]) -> Result<SharedPtr<Value>>;
        fn rt_greater_than(args: &[SharedPtr<Value>]) -> Result<SharedPtr<Value>>;
        fn rt_greater_than_or_equal(args: &[SharedPtr<Value>]) -> Result<SharedPtr<Value>>;
        fn rt_truthy(args: &[SharedPtr<Value>]) -> Result<SharedPtr<Value>>;
    }
}

// ---- Basic value passing ----

fn rt_identity(
    args: &[cxx::SharedPtr<ffi::Value>],
) -> Result<cxx::SharedPtr<ffi::Value>, String> {
    if args.is_empty() {
        return Err("expected at least 1 argument".into());
    }
    Ok(args[0].clone())
}

fn rt_type_name(
    args: &[cxx::SharedPtr<ffi::Value>],
) -> Result<cxx::SharedPtr<ffi::Value>, String> {
    if args.is_empty() {
        return Err("expected at least 1 argument".into());
    }
    let val = FrostValue::from_shared(args[0].clone());
    Ok(FrostValue::from_string(&val.type_name()).into_shared())
}

fn rt_round_trip(
    args: &[cxx::SharedPtr<ffi::Value>],
) -> Result<cxx::SharedPtr<ffi::Value>, String> {
    if args.is_empty() {
        return Err("expected at least 1 argument".into());
    }
    let val = FrostValue::from_shared(args[0].clone());

    let rebuilt = if let Some(i) = val.as_int() {
        FrostValue::from_int(i)
    } else if let Some(f) = val.as_float() {
        FrostValue::from_float(f).map_err(|e| e.to_string())?
    } else if let Some(b) = val.as_bool() {
        FrostValue::from_bool(b)
    } else if let Some(s) = val.as_string() {
        FrostValue::from_string(s)
    } else if val.is_null() {
        FrostValue::null()
    } else {
        return Err(format!("round_trip not supported for {}", val.type_name()));
    };

    Ok(rebuilt.into_shared())
}

// ---- Array operations ----

fn rt_array_sum(
    args: &[cxx::SharedPtr<ffi::Value>],
) -> Result<cxx::SharedPtr<ffi::Value>, String> {
    if args.is_empty() {
        return Err("expected an array argument".into());
    }
    let val = FrostValue::from_shared(args[0].clone());
    let elems = val.array_slice().ok_or("expected an Array")?;

    let mut sum: i64 = 0;
    for elem in elems {
        let v = FrostValue::from_shared(elem.clone());
        sum += v.as_int().ok_or("array element is not Int")?;
    }

    Ok(FrostValue::from_int(sum).into_shared())
}

fn rt_make_array(
    args: &[cxx::SharedPtr<ffi::Value>],
) -> Result<cxx::SharedPtr<ffi::Value>, String> {
    Ok(FrostValue::from_array(args).into_shared())
}

fn rt_array_reverse(
    args: &[cxx::SharedPtr<ffi::Value>],
) -> Result<cxx::SharedPtr<ffi::Value>, String> {
    if args.is_empty() {
        return Err("expected an array argument".into());
    }
    let val = FrostValue::from_shared(args[0].clone());
    let elems = val.array_slice().ok_or("expected an Array")?;

    let reversed: Vec<cxx::SharedPtr<ffi::Value>> = elems.iter().rev().cloned().collect();
    Ok(FrostValue::from_array(&reversed).into_shared())
}

// ---- Map operations ----

fn rt_make_map(
    args: &[cxx::SharedPtr<ffi::Value>],
) -> Result<cxx::SharedPtr<ffi::Value>, String> {
    if args.len() % 2 != 0 {
        return Err("expected even number of args (key, value pairs)".into());
    }
    let keys: Vec<_> = args.iter().step_by(2).cloned().collect();
    let values: Vec<_> = args.iter().skip(1).step_by(2).cloned().collect();
    let map = FrostValue::from_map(&keys, &values).map_err(|e| e.to_string())?;
    Ok(map.into_shared())
}

fn rt_map_keys_sorted(
    args: &[cxx::SharedPtr<ffi::Value>],
) -> Result<cxx::SharedPtr<ffi::Value>, String> {
    if args.is_empty() {
        return Err("expected a map argument".into());
    }
    let val = FrostValue::from_shared(args[0].clone());
    let keys = val.map_keys().ok_or("expected a Map")?;
    Ok(FrostValue::from_array(keys).into_shared())
}

fn rt_map_get_by_key(
    args: &[cxx::SharedPtr<ffi::Value>],
) -> Result<cxx::SharedPtr<ffi::Value>, String> {
    if args.len() < 2 {
        return Err("expected (map, key)".into());
    }
    let map = FrostValue::from_shared(args[0].clone());
    let key = FrostValue::from_shared(args[1].clone());
    match map.map_get_by(&key) {
        Some(v) => Ok(v.into_shared()),
        None => Ok(FrostValue::null().into_shared()),
    }
}

fn rt_map_entry_count(
    args: &[cxx::SharedPtr<ffi::Value>],
) -> Result<cxx::SharedPtr<ffi::Value>, String> {
    if args.is_empty() {
        return Err("expected a map argument".into());
    }
    let val = FrostValue::from_shared(args[0].clone());
    let len = val.map_len().ok_or("expected a Map")?;
    Ok(FrostValue::from_int(len as i64).into_shared())
}

// ---- String / binary operations ----

fn rt_concat_all(
    args: &[cxx::SharedPtr<ffi::Value>],
) -> Result<cxx::SharedPtr<ffi::Value>, String> {
    let mut result = String::new();
    for arg in args {
        let val = FrostValue::from_shared(arg.clone());
        result.push_str(val.as_string().ok_or("expected String")?);
    }
    Ok(FrostValue::from_string(&result).into_shared())
}

fn rt_string_byte_len(
    args: &[cxx::SharedPtr<ffi::Value>],
) -> Result<cxx::SharedPtr<ffi::Value>, String> {
    if args.is_empty() {
        return Err("expected a string argument".into());
    }
    let val = FrostValue::from_shared(args[0].clone());
    let bytes = val.as_bytes().ok_or("expected a String")?;
    Ok(FrostValue::from_int(bytes.len() as i64).into_shared())
}

// ---- Unpack (FrostRef enum) ----

fn rt_describe_type(
    args: &[cxx::SharedPtr<ffi::Value>],
) -> Result<cxx::SharedPtr<ffi::Value>, String> {
    if args.is_empty() {
        return Err("expected at least 1 argument".into());
    }
    let val = FrostValue::from_shared(args[0].clone());
    let desc = match val.unpack() {
        FrostRef::Null => "null".to_owned(),
        FrostRef::Int(n) => format!("int:{n}"),
        FrostRef::Float(f) => format!("float:{f}"),
        FrostRef::Bool(b) => format!("bool:{b}"),
        FrostRef::String(s) => format!("string:{}", s.as_bytes().len()),
        FrostRef::Array(elems) => format!("array:{}", elems.len()),
        FrostRef::Map { keys, .. } => format!("map:{}", keys.len()),
        FrostRef::Function(_) => "function".to_owned(),
    };
    Ok(FrostValue::from_string(&desc).into_shared())
}

// ---- Function operations ----

fn rt_call_callback(
    args: &[cxx::SharedPtr<ffi::Value>],
) -> Result<cxx::SharedPtr<ffi::Value>, String> {
    if args.len() < 2 {
        return Err("expected (function, arg)".into());
    }
    let func = FrostValue::from_shared(args[0].clone());
    let f = func.as_function().ok_or("expected Function")?;
    let result = f.call(&args[1..]).map_err(|e| e.to_string())?;
    Ok(result.into_shared())
}

fn rt_make_adder(
    args: &[cxx::SharedPtr<ffi::Value>],
) -> Result<cxx::SharedPtr<ffi::Value>, String> {
    if args.is_empty() {
        return Err("expected an int argument".into());
    }
    let n = FrostValue::from_shared(args[0].clone())
        .as_int()
        .ok_or("expected Int")?;

    let f = FrostFunction::new(move |call_args| {
        let x = call_args
            .first()
            .ok_or("expected 1 argument")?
            .as_int()
            .ok_or("expected Int")?;
        Ok(FrostValue::from_int(n + x))
    });
    Ok(f.into_shared())
}

fn rt_make_failing_fn(
    _args: &[cxx::SharedPtr<ffi::Value>],
) -> Result<cxx::SharedPtr<ffi::Value>, String> {
    let f = FrostFunction::new(|_| Err("intentional failure".to_owned()));
    Ok(f.into_shared())
}

// ---- Map null-value fidelity ----
// Returns Bool: true if map_get_by found the key (even if value is null),
// false if the key was not present.
fn rt_map_lookup_found(
    args: &[cxx::SharedPtr<ffi::Value>],
) -> Result<cxx::SharedPtr<ffi::Value>, String> {
    if args.len() < 2 {
        return Err("expected (map, key)".into());
    }
    let map = FrostValue::from_shared(args[0].clone());
    let key = FrostValue::from_shared(args[1].clone());
    let found = map.map_get_by(&key).is_some();
    Ok(FrostValue::from_bool(found).into_shared())
}

// ---- FrostArray::iter() ----

fn rt_array_iter_strings(
    args: &[cxx::SharedPtr<ffi::Value>],
) -> Result<cxx::SharedPtr<ffi::Value>, String> {
    if args.is_empty() {
        return Err("expected an array argument".into());
    }
    let val = FrostValue::from_shared(args[0].clone());
    let arr = val.as_array().ok_or("expected Array")?;

    let concatenated: String = arr.iter()
        .map(|v| v.to_display_string())
        .collect::<Vec<_>>()
        .join(",");

    Ok(FrostValue::from_string(&concatenated).into_shared())
}

// ---- Category checks ----

fn rt_is_numeric(
    args: &[cxx::SharedPtr<ffi::Value>],
) -> Result<cxx::SharedPtr<ffi::Value>, String> {
    if args.is_empty() { return Err("expected 1 argument".into()); }
    let val = FrostValue::from_shared(args[0].clone());
    Ok(FrostValue::from_bool(val.is_numeric()).into_shared())
}

fn rt_is_primitive(
    args: &[cxx::SharedPtr<ffi::Value>],
) -> Result<cxx::SharedPtr<ffi::Value>, String> {
    if args.is_empty() { return Err("expected 1 argument".into()); }
    let val = FrostValue::from_shared(args[0].clone());
    Ok(FrostValue::from_bool(val.is_primitive()).into_shared())
}

fn rt_is_structured(
    args: &[cxx::SharedPtr<ffi::Value>],
) -> Result<cxx::SharedPtr<ffi::Value>, String> {
    if args.is_empty() { return Err("expected 1 argument".into()); }
    let val = FrostValue::from_shared(args[0].clone());
    Ok(FrostValue::from_bool(val.is_structured()).into_shared())
}

// ---- serde deserialization ----

use frost_glue::de::from_value;

// Flat struct with required fields
#[derive(serde::Deserialize)]
struct FlatConfig {
    name: String,
    count: i64,
    enabled: bool,
}

fn rt_serde_flat_struct(
    args: &[cxx::SharedPtr<ffi::Value>],
) -> Result<cxx::SharedPtr<ffi::Value>, String> {
    let val = FrostValue::from_shared(args[0].clone());
    let config: FlatConfig = from_value(&val).map_err(|e| e.to_string())?;
    // Return a string proving we read all fields
    Ok(FrostValue::from_string(&format!(
        "{}:{}:{}",
        config.name, config.count, config.enabled
    ))
    .into_shared())
}

// Struct with optional/default fields
#[derive(serde::Deserialize)]
struct OptConfig {
    required: String,
    #[serde(default)]
    optional_int: i64,
    #[serde(default)]
    optional_str: String,
}

fn rt_serde_optional_field(
    args: &[cxx::SharedPtr<ffi::Value>],
) -> Result<cxx::SharedPtr<ffi::Value>, String> {
    let val = FrostValue::from_shared(args[0].clone());
    let config: OptConfig = from_value(&val).map_err(|e| e.to_string())?;
    Ok(FrostValue::from_string(&format!(
        "{}:{}:{}",
        config.required, config.optional_int, config.optional_str
    ))
    .into_shared())
}

// Nested struct
#[derive(serde::Deserialize)]
struct Inner {
    x: i64,
    y: i64,
}

#[derive(serde::Deserialize)]
struct Outer {
    label: String,
    point: Inner,
}

fn rt_serde_nested(
    args: &[cxx::SharedPtr<ffi::Value>],
) -> Result<cxx::SharedPtr<ffi::Value>, String> {
    let val = FrostValue::from_shared(args[0].clone());
    let outer: Outer = from_value(&val).map_err(|e| e.to_string())?;
    Ok(FrostValue::from_string(&format!(
        "{}:({},{})",
        outer.label, outer.point.x, outer.point.y
    ))
    .into_shared())
}

// Vec from Array
#[derive(serde::Deserialize)]
struct WithVec {
    items: Vec<String>,
}

fn rt_serde_vec(
    args: &[cxx::SharedPtr<ffi::Value>],
) -> Result<cxx::SharedPtr<ffi::Value>, String> {
    let val = FrostValue::from_shared(args[0].clone());
    let w: WithVec = from_value(&val).map_err(|e| e.to_string())?;
    Ok(FrostValue::from_string(&w.items.join(",")).into_shared())
}

// Direct primitive deserialization (not from a struct)
fn rt_serde_primitives(
    args: &[cxx::SharedPtr<ffi::Value>],
) -> Result<cxx::SharedPtr<ffi::Value>, String> {
    let val = FrostValue::from_shared(args[0].clone());
    // Deserialize the FrostValue as whatever it naturally is, then stringify
    // Try as i64 first, then f64, then bool, then string
    if val.is_int() {
        let n: i64 = from_value(&val).map_err(|e| e.to_string())?;
        Ok(FrostValue::from_string(&format!("int:{n}")).into_shared())
    } else if val.is_float() {
        let f: f64 = from_value(&val).map_err(|e| e.to_string())?;
        Ok(FrostValue::from_string(&format!("float:{f}")).into_shared())
    } else if val.is_bool() {
        let b: bool = from_value(&val).map_err(|e| e.to_string())?;
        Ok(FrostValue::from_string(&format!("bool:{b}")).into_shared())
    } else if val.is_string() {
        let s: String = from_value(&val).map_err(|e| e.to_string())?;
        Ok(FrostValue::from_string(&format!("string:{s}")).into_shared())
    } else if val.is_null() {
        let _: () = from_value(&val).map_err(|e| e.to_string())?;
        Ok(FrostValue::from_string("null").into_shared())
    } else {
        Err("unexpected type".into())
    }
}

// ---- require_args validation ----

use frost_glue::require::{self, Param, Type};

fn rt_require_nullary(
    args: &[cxx::SharedPtr<ffi::Value>],
) -> Result<cxx::SharedPtr<ffi::Value>, String> {
    let fv: Vec<_> = args.iter().map(|a| FrostValue::from_shared(a.clone())).collect();
    require::require_nullary("test.nullary", &fv)?;
    Ok(FrostValue::null().into_shared())
}

fn rt_require_one_int(
    args: &[cxx::SharedPtr<ffi::Value>],
) -> Result<cxx::SharedPtr<ffi::Value>, String> {
    let fv: Vec<_> = args.iter().map(|a| FrostValue::from_shared(a.clone())).collect();
    require::require_args("test.one_int", &fv, &[
        Param::required("value", &[Type::Int]),
    ])?;
    Ok(args[0].clone())
}

fn rt_require_string_opt_int(
    args: &[cxx::SharedPtr<ffi::Value>],
) -> Result<cxx::SharedPtr<ffi::Value>, String> {
    let fv: Vec<_> = args.iter().map(|a| FrostValue::from_shared(a.clone())).collect();
    require::require_args("test.str_opt_int", &fv, &[
        Param::required("name", &[Type::String]),
        Param::optional("count", &[Type::Int]),
    ])?;
    Ok(FrostValue::from_bool(true).into_shared())
}

fn rt_require_any(
    args: &[cxx::SharedPtr<ffi::Value>],
) -> Result<cxx::SharedPtr<ffi::Value>, String> {
    let fv: Vec<_> = args.iter().map(|a| FrostValue::from_shared(a.clone())).collect();
    require::require_args("test.any_val", &fv, &[
        Param::any("value"),
    ])?;
    Ok(args[0].clone())
}

// ---- Arithmetic / comparison / coercion ----
// Each takes two args (lhs, rhs) and applies the Frost-semantic operation.

fn binop(
    args: &[cxx::SharedPtr<ffi::Value>],
    op: impl Fn(&FrostValue, &FrostValue) -> Result<FrostValue, cxx::Exception>,
) -> Result<cxx::SharedPtr<ffi::Value>, String> {
    if args.len() < 2 {
        return Err("expected 2 arguments".into());
    }
    let lhs = FrostValue::from_shared(args[0].clone());
    let rhs = FrostValue::from_shared(args[1].clone());
    Ok(op(&lhs, &rhs).map_err(|e| e.to_string())?.into_shared())
}

fn rt_add(args: &[cxx::SharedPtr<ffi::Value>]) -> Result<cxx::SharedPtr<ffi::Value>, String> {
    binop(args, FrostValue::add)
}

fn rt_subtract(args: &[cxx::SharedPtr<ffi::Value>]) -> Result<cxx::SharedPtr<ffi::Value>, String> {
    binop(args, FrostValue::subtract)
}

fn rt_multiply(args: &[cxx::SharedPtr<ffi::Value>]) -> Result<cxx::SharedPtr<ffi::Value>, String> {
    binop(args, FrostValue::multiply)
}

fn rt_divide(args: &[cxx::SharedPtr<ffi::Value>]) -> Result<cxx::SharedPtr<ffi::Value>, String> {
    binop(args, FrostValue::divide)
}

fn rt_modulus(args: &[cxx::SharedPtr<ffi::Value>]) -> Result<cxx::SharedPtr<ffi::Value>, String> {
    binop(args, FrostValue::modulus)
}

fn rt_equal(args: &[cxx::SharedPtr<ffi::Value>]) -> Result<cxx::SharedPtr<ffi::Value>, String> {
    binop(args, FrostValue::equal)
}

fn rt_not_equal(args: &[cxx::SharedPtr<ffi::Value>]) -> Result<cxx::SharedPtr<ffi::Value>, String> {
    binop(args, FrostValue::not_equal)
}

fn rt_less_than(args: &[cxx::SharedPtr<ffi::Value>]) -> Result<cxx::SharedPtr<ffi::Value>, String> {
    binop(args, FrostValue::less_than)
}

fn rt_less_than_or_equal(args: &[cxx::SharedPtr<ffi::Value>]) -> Result<cxx::SharedPtr<ffi::Value>, String> {
    binop(args, FrostValue::less_than_or_equal)
}

fn rt_greater_than(args: &[cxx::SharedPtr<ffi::Value>]) -> Result<cxx::SharedPtr<ffi::Value>, String> {
    binop(args, FrostValue::greater_than)
}

fn rt_greater_than_or_equal(args: &[cxx::SharedPtr<ffi::Value>]) -> Result<cxx::SharedPtr<ffi::Value>, String> {
    binop(args, FrostValue::greater_than_or_equal)
}

fn rt_truthy(args: &[cxx::SharedPtr<ffi::Value>]) -> Result<cxx::SharedPtr<ffi::Value>, String> {
    if args.is_empty() {
        return Err("expected 1 argument".into());
    }
    let val = FrostValue::from_shared(args[0].clone());
    Ok(FrostValue::from_bool(val.truthy()).into_shared())
}
