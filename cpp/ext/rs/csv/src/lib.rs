use frost_glue::{FrostFunction, FrostMap, FrostValue};
use std::io::Read;

struct CsvOptions {
    delim: u8,
    has_headers: bool,
}

fn parse_options(options: FrostMap) -> Result<CsvOptions, String> {
    let mut delim = b',';
    let mut has_headers = true;

    let keys = options.keys();
    let values = options.values();
    for (k, v) in keys.iter().zip(values.iter()) {
        let key = FrostValue::from_shared(k.clone());
        let value = FrostValue::from_shared(v.clone());
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

type CsvReader = csv::Reader<Box<dyn Read>>;

fn each_csv(_reader: CsvReader, _callback: FrostFunction) -> Result<FrostValue, String> {
    Err("Placeholder".into())
}

fn record_to_frost_array(record: csv::StringRecord) -> FrostValue {
    let arr: Vec<_> = record.into_iter().map(FrostValue::from_string).collect();

    FrostValue::from_values(&arr)
}

fn record_to_frost_map(record: csv::StringRecord, headers: &[FrostValue]) -> FrostValue {
    let values: Vec<_> = record
        .into_iter()
        .map(FrostValue::from_string)
        .collect();

    FrostValue::from_entries_trusted(headers, &values)
}

fn collect_csv(mut reader: csv::Reader<Box<dyn Read>>) -> Result<FrostValue, String> {
    let headers = if reader.has_headers() {
        Some(reader.headers().map_err(|e| e.to_string())?.clone())
    } else {
        None
    };

    let header_keys: Option<Vec<_>> = headers.map(|h| {
        h.into_iter()
            .map(FrostValue::from_string)
            .collect()
    });

    let rows: Result<Vec<FrostValue>, String> = reader
        .records()
        .map(|result| {
            let record = result.map_err(|e| e.to_string())?;
            if let Some(ref header_keys) = header_keys {
                Ok(record_to_frost_map(record, header_keys))
            } else {
                Ok(record_to_frost_array(record))
            }
        })
        .collect();

    rows.map(|r| FrostValue::from_values(&r))
}

fn read_csv(reader: CsvReader, callback: Option<FrostFunction>) -> Result<FrostValue, String> {
    if let Some(callback) = callback {
        each_csv(reader, callback)
    } else {
        collect_csv(reader)
    }
}

fn read_file(
    path: &str,
    options: FrostMap,
    callback: Option<FrostFunction>,
) -> Result<FrostValue, String> {
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let source: Box<dyn Read> = Box::new(file);

    let options = parse_options(options)?;

    let reader = csv::ReaderBuilder::new()
        .delimiter(options.delim)
        .has_headers(options.has_headers)
        .from_reader(source);

    read_csv(reader, callback)
}

include!(concat!(env!("OUT_DIR"), "/bridge.rs"));
