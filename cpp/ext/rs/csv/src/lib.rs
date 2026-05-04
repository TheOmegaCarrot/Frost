use frost_glue::{
    FrostFunction, FrostMap, FrostValue, de,
    require::{Param, Type, require_args, require_nullary},
};
use std::{
    fs::File,
    io::{Cursor, Read, Write},
    sync::{Arc, Mutex},
};

fn prefix_err(prefix: &str, err: String) -> String {
    format!("{prefix}: {err}")
}

// =======================================
// Reading CSV
// =======================================

fn default_delim() -> u8 {
    b','
}

fn deserialize_delim<'de, D: serde::Deserializer<'de>>(de: D) -> Result<u8, D::Error> {
    let s: String = serde::Deserialize::deserialize(de)?;
    if let [b] = s.as_bytes() {
        Ok(*b)
    } else {
        Err(serde::de::Error::custom("delimiter must be a single byte"))
    }
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadCsvOptions {
    #[serde(default = "default_delim", deserialize_with = "deserialize_delim")]
    delim: u8,
    headers: bool,
}

type CsvReader = csv::Reader<Box<dyn Read>>;

fn record_to_frost_array(record: csv::StringRecord) -> FrostValue {
    let arr: Vec<_> = record.into_iter().map(FrostValue::from_string).collect();

    FrostValue::from_values(&arr)
}

fn record_to_frost_map(record: csv::StringRecord, headers: &[FrostValue]) -> FrostValue {
    let values: Vec<_> = record.into_iter().map(FrostValue::from_string).collect();

    FrostValue::from_entries_trusted(headers, &values)
}

fn extract_headers(reader: &mut CsvReader) -> Result<Option<Vec<FrostValue>>, String> {
    if reader.has_headers() {
        let headers = reader.headers().map_err(|e| e.to_string())?;
        Ok(Some(
            headers.into_iter().map(FrostValue::from_string).collect(),
        ))
    } else {
        Ok(None)
    }
}

fn process_csv(
    mut reader: CsvReader,
    mut on_row: impl FnMut(FrostValue) -> Result<FrostValue, String>,
) -> Result<Vec<FrostValue>, String> {
    let headers = extract_headers(&mut reader)?;

    reader
        .records()
        .map(|result| {
            let record = result.map_err(|e| e.to_string())?;
            let row = if let Some(ref headers) = headers {
                record_to_frost_map(record, headers)
            } else {
                record_to_frost_array(record)
            };
            on_row(row)
        })
        .collect()
}

fn read_csv(
    source: Box<dyn Read>,
    options: FrostMap,
    callback: Option<FrostFunction>,
) -> Result<FrostValue, String> {
    let options: ReadCsvOptions =
        de::from_value(&options.into_value()).map_err(|e| e.to_string())?;

    let reader = csv::ReaderBuilder::new()
        .delimiter(options.delim)
        .has_headers(options.headers)
        .from_reader(source);

    let rows = if let Some(callback) = callback {
        process_csv(reader, |row| callback.call_with(&[row]))?
    } else {
        process_csv(reader, Ok)?
    };

    Ok(FrostValue::from_values(&rows))
}

// =======================================
// Writing CSV
// =======================================

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields, default)]
struct WriteCsvOptions {
    headers: Option<Vec<String>>,
    #[serde(deserialize_with = "deserialize_delim")]
    delim: u8,
}

impl Default for WriteCsvOptions {
    fn default() -> Self {
        Self {
            headers: None,
            delim: default_delim(),
        }
    }
}

fn parse_write_options(options: Option<FrostMap>) -> Result<WriteCsvOptions, String> {
    if let Some(options) = options {
        let val = options.into_value();
        Ok(de::from_value(&val).map_err(|e|e.to_string())?)
    } else {
        Ok(WriteCsvOptions::default())
    }
}

fn frost_value_to_record(
    value: &FrostValue,
    headers: &Option<Vec<String>>,
) -> Result<Vec<String>, String> {
    if let Some(arr) = value.as_array() {
        arr.iter()
            .map(|val| {
                if !val.is_primitive() {
                    let tn = val.type_name();
                    return Err(format!("row values must be primitives, got {tn}"));
                }
                Ok(val.to_display_string())
            })
            .collect::<Result<Vec<String>, String>>()
    } else if let Some(map) = value.as_map() {
        let headers = headers
            .as_ref()
            .ok_or("Map rows require headers to be defined")?;

        headers
            .iter()
            .map(|h| {
                Ok(map
                    .get(h)
                    .ok_or(format!("Map is missing key '{h}'"))?
                    .to_display_string())
            })
            .collect()
    } else {
        Err(format!("expected Array or Map, got {}", value.type_name()))
    }
}

fn make_row_fn<W: Write + Send + 'static>(
    writer: &Arc<Mutex<csv::Writer<W>>>,
    headers: Option<Vec<String>>,
) -> FrostValue {
    let w = writer.clone();
    FrostFunction::new(move |args| {
        require_args(
            "csv.writer.row",
            args,
            &[Param::required("row", &[Type::Array, Type::Map])],
        )?;
        {
            let mut writer = w.lock().map_err(|e| e.to_string())?;
            let row = frost_value_to_record(&args[0], &headers)?;
            writer.write_record(&row).map_err(|e| e.to_string())?;
            Ok(FrostValue::null())
        }
        .map_err(|e| prefix_err("csv.writer.row", e))
    })
    .into_value()
}

fn make_flush_fn(writer: &Arc<Mutex<csv::Writer<File>>>) -> FrostValue {
    let w = writer.clone();
    FrostFunction::new(move |args| {
        require_nullary("csv.writer.flush", args)?;
        {
            let mut writer = w.lock().map_err(|e| e.to_string())?;
            writer.flush().map_err(|e| e.to_string())?;
            Ok(FrostValue::null())
        }
        .map_err(|e| prefix_err("csv.writer.flush", e))
    })
    .into_value()
}

fn make_get_fn(writer: &Arc<Mutex<csv::Writer<Vec<u8>>>>) -> FrostValue {
    let w = writer.clone();
    FrostFunction::new(move |args| {
        require_nullary("csv.writer.get", args)?;
        {
            let mut writer = w.lock().map_err(|e| e.to_string())?;
            writer.flush().map_err(|e| e.to_string())?;
            let s = std::str::from_utf8(writer.get_ref()).map_err(|e| e.to_string())?;
            Ok(FrostValue::from_string(s))
        }
        .map_err(|e| prefix_err("csv.writer.get", e))
    })
    .into_value()
}

fn make_file_writer_bundle(
    writer: csv::Writer<File>,
    headers: Option<Vec<String>>,
) -> Result<FrostValue, String> {
    let writer = Arc::new(Mutex::new(writer));

    Ok(FrostValue::from_pairs_trusted(&[
        (
            FrostValue::from_string("row"),
            make_row_fn(&writer, headers),
        ),
        (FrostValue::from_string("flush"), make_flush_fn(&writer)),
    ]))
}

fn make_string_writer_bundle(
    writer: csv::Writer<Vec<u8>>,
    headers: Option<Vec<String>>,
) -> Result<FrostValue, String> {
    let writer = Arc::new(Mutex::new(writer));

    Ok(FrostValue::from_pairs_trusted(&[
        (
            FrostValue::from_string("row"),
            make_row_fn(&writer, headers),
        ),
        (FrostValue::from_string("get"), make_get_fn(&writer)),
    ]))
}

// =======================================
// Entry points
// =======================================
fn read_file(
    path: &str,
    options: FrostMap,
    callback: Option<FrostFunction>,
) -> Result<FrostValue, String> {
    {
        let file = File::open(path).map_err(|e| e.to_string())?;
        let source: Box<dyn Read> = Box::new(file);
        read_csv(source, options, callback)
    }
    .map_err(|e| prefix_err("csv.read_file", e))
}

fn read_str(
    text: &str,
    options: FrostMap,
    callback: Option<FrostFunction>,
) -> Result<FrostValue, String> {
    {
        let cursor = Cursor::new(text.to_owned().into_bytes());
        let source: Box<dyn Read> = Box::new(cursor);
        read_csv(source, options, callback)
    }
    .map_err(|e| prefix_err("csv.read_str", e))
}

fn write_file(path: &str, options: Option<FrostMap>) -> Result<FrostValue, String> {
    {
        let options = parse_write_options(options)?;
        let mut writer = csv::WriterBuilder::new()
            .delimiter(options.delim)
            .from_path(path)
            .map_err(|e| e.to_string())?;

        if let Some(ref headers) = options.headers {
            writer.write_record(headers).map_err(|e| e.to_string())?;
        }

        make_file_writer_bundle(writer, options.headers)
    }
    .map_err(|e| prefix_err("csv.write_file", e))
}

fn write_str(options: Option<FrostMap>) -> Result<FrostValue, String> {
    {
        let options = parse_write_options(options)?;
        let mut writer = csv::WriterBuilder::new()
            .delimiter(options.delim)
            .from_writer(Vec::<u8>::new());

        if let Some(ref headers) = options.headers {
            writer.write_record(headers).map_err(|e| e.to_string())?;
        }

        make_string_writer_bundle(writer, options.headers)
    }
    .map_err(|e| prefix_err("csv.write_str", e))
}

include!(concat!(env!("OUT_DIR"), "/bridge.rs"));
