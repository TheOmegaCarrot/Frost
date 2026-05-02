use frost_glue::FrostValue;

fn add_one(value: i64) -> Result<FrostValue, String> {
    Ok(FrostValue::from_int(value + 1))
}

fn negate_float(value: f64) -> Result<FrostValue, String> {
    FrostValue::from_float(-value).map_err(|e| e.to_string())
}

fn flip(value: bool) -> Result<FrostValue, String> {
    Ok(FrostValue::from_bool(!value))
}

fn shout(value: &str) -> Result<FrostValue, String> {
    Ok(FrostValue::from_string(&value.to_uppercase()))
}

fn to_float(value: FrostValue) -> Result<FrostValue, String> {
    let f = value
        .as_float()
        .or_else(|| value.as_int().map(|i| i as f64))
        .ok_or("expected Int or Float")?;
    FrostValue::from_float(f).map_err(|e| e.to_string())
}

fn identity(value: FrostValue) -> Result<FrostValue, String> {
    Ok(value)
}

fn greet(name: &str, greeting: Option<&str>) -> Result<FrostValue, String> {
    let g = greeting.unwrap_or("hello");
    Ok(FrostValue::from_string(&format!("{g}, {name}!")))
}

fn concat(parts: &[FrostValue]) -> Result<FrostValue, String> {
    let mut result = String::new();
    for part in parts {
        result.push_str(part.as_string().ok_or("expected String")?);
    }
    Ok(FrostValue::from_string(&result))
}

fn forty_two() -> Result<FrostValue, String> {
    Ok(FrostValue::from_int(42))
}

fn fail_if_negative(value: i64) -> Result<FrostValue, String> {
    if value < 0 {
        Err(format!("negative value: {value}"))
    } else {
        Ok(FrostValue::from_int(value))
    }
}

fn array_sum(arr: FrostValue) -> Result<FrostValue, String> {
    let elems = arr.array_slice().ok_or("expected Array")?;
    let mut sum: i64 = 0;
    for elem in elems {
        let v = FrostValue::from_shared(elem.clone());
        sum += v.as_int().ok_or("array element is not Int")?;
    }
    Ok(FrostValue::from_int(sum))
}

fn make_adder(n: i64) -> Result<FrostValue, String> {
    let f = frost_glue::FrostFunction::new(move |args| {
        let x = args.first().ok_or("expected 1 argument")?
            .as_int().ok_or("expected Int")?;
        Ok(FrostValue::from_int(n + x))
    });
    Ok(f.into_value())
}

use std::sync::atomic::{AtomicI64, Ordering};

struct Counter {
    value: AtomicI64,
}

fn make_counter(start: i64) -> Result<FrostValue, String> {
    Ok(make_data_counter(
        Counter { value: AtomicI64::new(start) },
        |counter, _args| {
            let prev = counter.value.fetch_add(1, Ordering::Relaxed);
            Ok(FrostValue::from_int(prev))
        },
    ))
}

fn counter_value(counter_fn: FrostValue) -> Result<FrostValue, String> {
    let counter = try_downcast_counter::<Counter>(&counter_fn)
        .ok_or("not a Counter")?;
    Ok(FrostValue::from_int(counter.value.load(Ordering::Relaxed)))
}

fn apply(func: FrostValue, value: FrostValue) -> Result<FrostValue, String> {
    let f = func.as_function().ok_or("expected Function")?;
    f.call_with(&[value]).map_err(|e| e.to_string())
}

include!(concat!(env!("OUT_DIR"), "/bridge.rs"));
