use frost_glue::{FrostFunction, FrostMap, FrostValue};
use std::io::{Cursor, Read};

// =======================================
// Generic to reading and writing
// =======================================
struct CsvOptions {
    delim: u8,
    has_headers: bool,
}

fn parse_options(options: FrostMap) -> Result<CsvOptions, String> {
    let mut delim = b',';
    let mut has_headers = true;

    for (key, value) in options.iter() {
        match key.as_string() {
            Some("delim") => {
                let bytes = value.as_bytes().ok_or("csv: delimiter must be a String")?;
                if bytes.len() != 1 {
                    return Err("csv: delimiter must be a single byte".into());
                }
                delim = bytes[0];
            }
            Some("headers") => {
                if !value.is_bool() {
                    return Err("csv: headers option must be a Bool".into());
                }
                has_headers = value.as_bool().unwrap();
            }
            Some(other) => {
                return Err(format!("csv: unknown option: {other}"));
            }
            None => return Err("csv: option keys must be Strings".into()),
        }
    }

    Ok(CsvOptions { delim, has_headers })
}

// =======================================
// Reading CSV
// =======================================

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
    let options = parse_options(options)?;

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

include!(concat!(env!("OUT_DIR"), "/bridge.rs"));
