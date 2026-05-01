use frost_glue::FrostValue;

fn concat(a: &str, b: &str) -> Result<FrostValue, String> {
    Ok(FrostValue::from_string(&format!("{a}{b}")))
}

include!(concat!(env!("OUT_DIR"), "/bridge.rs"));
