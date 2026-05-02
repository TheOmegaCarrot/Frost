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
