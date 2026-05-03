use frost_glue::{
    FrostArray, FrostFunction, FrostMap, FrostRef, FrostValue,
    require::{Param, Type, require_args, require_nullary},
};
use std::{
    fs::File,
    io::{Cursor, Read, Write},
    sync::{Arc, Mutex},
};

// =======================================
// Reading CSV
// =======================================

struct ReadCsvOptions {
    delim: u8,
    has_headers: bool,
}

fn parse_read_options(options: FrostMap) -> Result<ReadCsvOptions, String> {
    let mut delim = b',';
    let mut has_headers: Option<bool> = None;

    for (key, value) in options.iter() {
        match key.as_string() {
            Some("delim") => {
                let Some(&[byte]) = value.as_bytes() else {
                    return Err("csv: delimiter must be a single-byte String".into());
                };
                delim = byte;
            }
            Some("headers") => {
                if !value.is_bool() {
                    return Err("csv: headers option must be a Bool".into());
                }
                has_headers = value.as_bool();
            }
            Some(other) => {
                return Err(format!("csv: unknown option: {other}"));
            }
            None => return Err("csv: option keys must be Strings".into()),
        }
    }

    let has_headers = has_headers.ok_or("csv: 'headers' option is required")?;

    Ok(ReadCsvOptions { delim, has_headers })
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
    let options = parse_read_options(options)?;

    let reader = csv::ReaderBuilder::new()
        .delimiter(options.delim)
        .has_headers(options.has_headers)
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

struct WriteCsvOptions {
    headers: Option<Vec<String>>,
    delim: u8,
}

impl Default for WriteCsvOptions {
    fn default() -> Self {
        Self {
            headers: None,
            delim: b',',
        }
    }
}

fn parse_write_options(options: Option<FrostMap>) -> Result<WriteCsvOptions, String> {
    let mut opts = WriteCsvOptions::default();

    let Some(options) = options else {
        return Ok(opts);
    };

    for (key, value) in options.iter() {
        match key.as_string() {
            Some("delim") => {
                let Some(&[byte]) = value.as_bytes() else {
                    return Err("csv: delimiter must be a single-byte String".into());
                };
                opts.delim = byte;
            }
            Some("headers") => {
                opts.headers = Some(frost_array_to_string_vec(
                    value
                        .as_array()
                        .ok_or("csv: 'headers' option must be an Array")?,
                )?);
            }
            Some(other) => {
                return Err(format!("csv: unknown option: {other}"));
            }
            None => return Err("csv: option keys must be Strings".into()),
        }
    }

    Ok(opts)
}

fn frost_array_to_string_vec(headers: FrostArray) -> Result<Vec<String>, String> {
    headers
        .iter()
        .map(|elem| {
            Ok(elem
                .as_string()
                .ok_or("csv: 'headers' array must only contain Strings")?
                .to_owned())
        })
        .collect()
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
                    return Err(format!(
                        "csv.writer.row: Row values must be primitives, got {tn}"
                    ));
                }
                Ok(val.to_display_string())
            })
            .collect::<Result<Vec<String>, String>>()
    } else if let Some(map) = value.as_map() {
        let headers = headers
            .as_ref()
            .ok_or("csv.writer.row: Map rows require headers to be defined")?;

        headers
            .iter()
            .map(|h| {
                Ok(map
                    .get(h)
                    .ok_or(format!("csv.writer.row: Map is missing key '{h}'"))?
                    .to_display_string())
            })
            .collect()
    } else {
        Err(format!(
            "csv.writer.row: expected Array or Map, got {}",
            value.type_name()
        ))
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
        let mut writer = w.lock().map_err(|e| e.to_string())?;

        let row = frost_value_to_record(&args[0], &headers)?;
        writer.write_record(&row).map_err(|e| e.to_string())?;

        Ok(FrostValue::null())
    })
    .into_value()
}

fn make_file_writer_bundle(
    writer: csv::Writer<File>,
    headers: Option<Vec<String>>,
) -> Result<FrostValue, String> {
    let writer = Arc::new(Mutex::new(writer));

    let row_fn = make_row_fn(&writer, headers);

    let flush_fn = {
        let w = writer.clone();
        FrostFunction::new(move |args| {
            require_nullary("csv.writer.flush", args)?;
            let mut writer = w.lock().map_err(|e| e.to_string())?;
            writer.flush().map_err(|e| e.to_string())?;
            Ok(FrostValue::null())
        })
        .into_value()
    };

    Ok(FrostValue::from_pairs_trusted(&[
        (FrostValue::from_string("row"), row_fn),
        (FrostValue::from_string("flush"), flush_fn),
    ]))
}

fn make_string_writer_bundle(
    writer: csv::Writer<Vec<u8>>,
    headers: Option<Vec<String>>,
) -> Result<FrostValue, String> {
    let writer = Arc::new(Mutex::new(writer));

    let row_fn = make_row_fn(&writer, headers);

    let get_fn = {
        let w = writer.clone();
        FrostFunction::new(move |args| {
            require_nullary("csv.writer.get", args)?;
            let mut writer = w.lock().map_err(|e| e.to_string())?;
            writer.flush().map_err(|e| e.to_string())?;
            let s = std::str::from_utf8(writer.get_ref()).map_err(|e| e.to_string())?;

            Ok(FrostValue::from_string(s))
        })
        .into_value()
    };

    Ok(FrostValue::from_pairs_trusted(&[
        (FrostValue::from_string("row"), row_fn),
        (FrostValue::from_string("get"), get_fn),
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
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let source: Box<dyn Read> = Box::new(file);

    read_csv(source, options, callback)
}

fn read_str(
    csv: &str,
    options: FrostMap,
    callback: Option<FrostFunction>,
) -> Result<FrostValue, String> {
    let cursor = Cursor::new(csv.to_owned().into_bytes());
    let source: Box<dyn Read> = Box::new(cursor);

    read_csv(source, options, callback)
}

fn write_file(path: &str, options: Option<FrostMap>) -> Result<FrostValue, String> {
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

fn write_str(options: Option<FrostMap>) -> Result<FrostValue, String> {
    let options = parse_write_options(options)?;
    let mut writer = csv::WriterBuilder::new()
        .delimiter(options.delim)
        .from_writer(Vec::<u8>::new());

    if let Some(ref headers) = options.headers {
        writer.write_record(headers).map_err(|e| e.to_string())?;
    }

    make_string_writer_bundle(writer, options.headers)
}

include!(concat!(env!("OUT_DIR"), "/bridge.rs"));
