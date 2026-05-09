use frost_glue::{FrostMap, FrostValue, de};
use ini::Ini;

// =======================================
// Encode
// =======================================

#[derive(serde::Deserialize)]
enum LineSep {
    #[serde(alias = "LF", alias = "lf", alias = "\n")]
    Lf,
    #[serde(alias = "CRLF", alias = "crlf", alias = "\r\n")]
    CrLf,
}

impl From<LineSep> for ini::LineSeparator {
    fn from(ls: LineSep) -> Self {
        match ls {
            LineSep::Lf => ini::LineSeparator::CR,
            LineSep::CrLf => ini::LineSeparator::CRLF,
        }
    }
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields, default)]
struct IniEncodeOptions {
    kv_separator: String,
    line_separator: LineSep,
}

impl Default for IniEncodeOptions {
    fn default() -> Self {
        Self {
            kv_separator: " = ".into(),
            line_separator: LineSep::Lf,
        }
    }
}

fn static_kv_sep(kv_sep: String) -> Result<&'static str, String> {
    Ok(match kv_sep.as_str() {
        "=" => "=",
        " =" => " =",
        "= " => "= ",
        " = " => " = ",
        ":" => ":",
        ": " => ": ",
        " :" => " :",
        " : " => " : ",
        _ => return Err("INI kv_separator is invalid".into()),
    })
}

impl TryFrom<IniEncodeOptions> for ini::WriteOption {
    type Error = String;

    fn try_from(val: IniEncodeOptions) -> Result<Self, String> {
        Ok(Self {
            line_separator: val.line_separator.into(),
            kv_separator: static_kv_sep(val.kv_separator)?,
            escape_policy: ini::EscapePolicy::Reserved,
        })
    }
}

fn parse_encode_options(options: Option<FrostMap>) -> Result<IniEncodeOptions, String> {
    if let Some(options) = options {
        de::from_value(&options.into_value()).map_err(|e| e.to_string())
    } else {
        Ok(IniEncodeOptions::default())
    }
}

fn write_section(section: &mut ini::SectionSetter<'_>, map: FrostValue) -> Result<(), String> {
    let Some(map) = map.as_map() else {
        return Err(format!(
            "INI section must be a Map, got: {}",
            map.to_display_string()
        ));
    };

    for (k, v) in map.iter() {
        section.set(
            k.as_string()
                .ok_or(format!("INI keys must be Strings, got: {}", k.type_name()))?,
            v.as_string().ok_or(format!(
                "INI values must be Strings, got: {}",
                v.type_name()
            ))?,
        );
    }

    Ok(())
}

fn write_sections(ini: &mut Ini, sections: FrostValue) -> Result<(), String> {
    let Some(sections) = sections.as_map() else {
        return Err(format!(
            "INI sections must be a Map, got: {}",
            sections.type_name()
        ));
    };

    for (name, section) in sections.iter() {
        let Some(name) = name.as_string() else {
            return Err(format!(
                "INI section names must be Strings, got: {}",
                name.type_name()
            ));
        };

        write_section(&mut ini.with_section(Some(name)), section)?;
    }

    Ok(())
}

fn frost_to_ini_str(ini_map: FrostMap, opts: ini::WriteOption) -> Result<String, String> {
    let mut ini = Ini::new();

    for (key, value) in ini_map.iter() {
        match key.as_string() {
            Some("globals") => write_section(&mut ini.with_section(None::<String>), value)?,
            Some("sections") => write_sections(&mut ini, value)?,
            _ => {
                return Err(format!(
                    "INI document must have String keys 'globals' or 'sections', got: {}",
                    key.to_display_string()
                ));
            }
        }
    }

    let mut buf = Vec::new();
    ini.write_to_opt(&mut buf, opts)
        .map_err(|e| e.to_string())?;
    String::from_utf8(buf).map_err(|e| e.to_string())
}

// =======================================
// Decode
// =======================================

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields, default)]
struct IniDecodeOptions {
    strip_quotes: bool,
    escapes: bool,
    multiline_values: bool,
}

impl Default for IniDecodeOptions {
    fn default() -> Self {
        Self {
            strip_quotes: true,
            escapes: true,
            multiline_values: false,
        }
    }
}

// IniDecodeOptions is the Frost-facing options,
// while ini::ParseOption is what the INI library provides
impl From<IniDecodeOptions> for ini::ParseOption {
    fn from(val: IniDecodeOptions) -> Self {
        Self {
            enabled_quote: val.strip_quotes,
            enabled_escape: val.escapes,
            enabled_indented_mutiline_value: val.multiline_values,
            ..ini::ParseOption::default()
        }
    }
}

fn parse_decode_options(opts: Option<FrostMap>) -> Result<IniDecodeOptions, String> {
    if let Some(opts) = opts {
        de::from_value(&opts.into_value()).map_err(|e| e.to_string())
    } else {
        Ok(IniDecodeOptions::default())
    }
}

fn section_to_frost(section: &ini::Properties) -> FrostValue {
    FrostValue::from_pairs_trusted_iter(section.iter().map(|(k, v)| (k.into(), v.into())))
}

fn ini_to_frost(ini: Ini) -> FrostValue {
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

fn encode(ini: FrostMap, options: Option<FrostMap>) -> Result<FrostValue, String> {
    let options = parse_encode_options(options)?;

    Ok(frost_to_ini_str(ini, options.try_into()?)?.into())
}

fn decode(ini_str: &str, options: Option<FrostMap>) -> Result<FrostValue, String> {
    let options = parse_decode_options(options)?;
    let ini = Ini::load_from_str_opt(ini_str, options.into()).map_err(|e| e.to_string())?;

    Ok(ini_to_frost(ini))
}

include!(concat!(env!("OUT_DIR"), "/bridge.rs"));
