use frost_glue::FrostValue;

#[cxx::bridge(namespace = "frst::rs::test")]
mod ffi {
    unsafe extern "C++" {
        include!("frost-glue.hpp");
        #[namespace = "frst::rs"]
        type Value = frost_glue::ffi::Value;
    }

    extern "Rust" {
        // C++ passes values in, Rust inspects and returns results.

        // Identity: receive a value, return it unchanged.
        fn rt_identity(args: &[SharedPtr<Value>]) -> Result<SharedPtr<Value>>;

        // Type name: return the type name as a String value.
        fn rt_type_name(args: &[SharedPtr<Value>]) -> Result<SharedPtr<Value>>;

        // Extract and reconstruct: pull out the inner data and rebuild.
        fn rt_round_trip(args: &[SharedPtr<Value>]) -> Result<SharedPtr<Value>>;

        // Array sum: given an array of Ints, return their sum.
        fn rt_array_sum(args: &[SharedPtr<Value>]) -> Result<SharedPtr<Value>>;

        // Build a string from parts: concatenate all string args.
        fn rt_concat_all(args: &[SharedPtr<Value>]) -> Result<SharedPtr<Value>>;
    }
}

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
