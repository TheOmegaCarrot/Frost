use frost_glue::{FrostMap, FrostValue, de};

// =======================================
// Encode
// =======================================



// =======================================
// Decode
// =======================================

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields, default)]
struct IniParseOptions {
    strip_quotes: bool,
    escapes: bool,
    multiline_values: bool,
}

impl Default for IniParseOptions {
    fn default() -> Self {
        Self {
            strip_quotes: true,
            escapes: true,
            multiline_values: false,
        }
    }
}

// IniParseOptions is the Frost-facing options,
// while ini::ParseOption is what the INI library provides
impl From<IniParseOptions> for ini::ParseOption {
    fn from(val: IniParseOptions) -> Self {
        ini::ParseOption {
            enabled_quote: val.strip_quotes,
            enabled_escape: val.escapes,
            enabled_indented_mutiline_value: val.multiline_values,
            ..ini::ParseOption::default()
        }
    }
}

fn parse_decode_options(opts: Option<FrostMap>) -> Result<IniParseOptions, String> {
    if let Some(opts) = opts {
        de::from_value(&opts.into_value()).map_err(|e| e.to_string())
    } else {
        Ok(IniParseOptions::default())
    }
}

fn section_to_frost(section: &ini::Properties) -> FrostValue {
    FrostValue::from_pairs_trusted_iter(
        section
            .iter()
            .map(|(k, v)| (k.into(), v.into()))
    )
}

fn ini_to_frost(ini: ini::Ini) -> FrostValue {
    let globals = section_to_frost(ini.general_section());

    let mut sections = Vec::<(FrostValue, FrostValue)>::new();
    for (section, props) in ini.iter() {
        match section {
            None => {} // general section handled separately
            Some(section_name) => sections.push((section_name.into(), section_to_frost(props))),
        }
    }

    let sections = FrostValue::from_pairs_trusted(sections.as_slice());

    FrostValue::from_pairs_trusted(&[("globals".into(), globals), ("sections".into(), sections)])
}

// =======================================
// Entry points
// =======================================

fn encode(_ini: FrostMap, _options: Option<FrostMap>) -> Result<FrostValue, String> {
    todo!()
}

fn decode(ini_str: &str, options: Option<FrostMap>) -> Result<FrostValue, String> {
    let options = parse_decode_options(options)?;
    let ini = ini::Ini::load_from_str_opt(ini_str, options.into()).map_err(|e| e.to_string())?;

    Ok(ini_to_frost(ini))
}

include!(concat!(env!("OUT_DIR"), "/bridge.rs"));
