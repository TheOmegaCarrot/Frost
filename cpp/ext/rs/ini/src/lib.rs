use frost_glue::{
    FrostMap, FrostValue, de,
};
use std::io::Cursor;


// =======================================
// Entry points
// =======================================


fn encode(_ini: FrostMap, _options: Option<FrostMap>) -> Result<FrostValue, String> {
    todo!()
}

fn decode(_ini: &str, _options: Option<FrostMap>) -> Result<FrostValue, String> {
    todo!()
}


include!(concat!(env!("OUT_DIR"), "/bridge.rs"));
