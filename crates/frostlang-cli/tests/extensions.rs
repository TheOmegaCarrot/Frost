//! The first-party extensions `frost` installs.

use std::process::Command;

/// What `frost -e code` prints, which must succeed.
fn eval(code: &str) -> String {
    eval_with(code, Command::new(env!("CARGO_BIN_EXE_frost")))
}

/// What `frost -e code` prints with the system time zone set to `tz`, which
/// must succeed. Only the child's environment changes.
fn eval_in_zone(code: &str, tz: &str) -> String {
    let mut command = Command::new(env!("CARGO_BIN_EXE_frost"));
    command.env("TZ", tz);
    eval_with(code, command)
}

/// What `command`, running `frost`, prints for `-e code`, which must succeed.
fn eval_with(code: &str, mut command: Command) -> String {
    let output = command
        .args(["-e", code])
        .output()
        .expect("`frost` should run");
    assert!(
        output.status.success(),
        "{code:?} should succeed, but failed with: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("the output is UTF-8")
}

#[test]
fn uuid_is_installed() {
    let code = r"
        def uuid = import('ext.uuid')
        print(uuid.version(uuid.v4()))
    ";
    assert_eq!(eval(code), "4\n");
}

#[test]
fn datetime_is_installed() {
    let code = r"
        def dt = import('ext.datetime')
        print(dt.instant('1970-01-01T00:00:01Z'))
    ";
    assert_eq!(eval(code), "1000000\n");
}

#[test]
fn datetime_uses_the_system_time_zone_by_default() {
    // 2024-03-16T14:30:00Z is 10:30 in New York.
    let code = r"
        def dt = import('ext.datetime')
        print(dt.local_zone())
        print(dt.to_rfc9557(1710599400000000))
        print(dt.instant({year: 2024, month: 3, day: 16, hour: 10, minute: 30}))
        print(dt.format(1710599400000000, '%H:%M %z'))
        print(dt.weekday(1710644400000000))
    ";
    assert_eq!(
        eval_in_zone(code, "America/New_York"),
        "America/New_York\n2024-03-16T10:30:00-04:00[America/New_York]\n1710599400000000\n\
         10:30 -0400\n6\n",
        "each default is New York's; 2024-03-17T03:00Z, a Sunday in UTC, is Saturday there"
    );
}

#[test]
fn datetime_in_a_system_zone_without_a_name_gives_offsets() {
    let code = r"
        def dt = import('ext.datetime')
        print(dt.local_zone())
        print(dt.to_rfc9557(1710599400000000))
        def components = dt.components(1710599400000000)
        print([has(components, 'offset'), has(components, 'zone')])
    ";
    assert_eq!(
        eval_in_zone(code, "EST5EDT,M3.2.0,M11.1.0"),
        "null\n2024-03-16T10:30:00-04:00\n[ true, false ]\n",
        "a POSIX rule names no zone, so components have an offset and no zone"
    );
}

#[test]
fn formats_is_installed_with_every_format() {
    let code = r"
        print(sorted(keys(import('ext.formats'))) == ['cbor', 'json', 'msgpack', 'toml', 'u64', 'yaml'])
    ";
    assert_eq!(eval(code), "true\n");
}

#[test]
fn formats_json_round_trips() {
    let code = r#"
        def json = import('ext.formats.json')
        print(json.encode(json.decode('{"a": [1]}')) == '{"a":[1]}')
    "#;
    assert_eq!(eval(code), "true\n");
}

#[test]
fn formats_toml_round_trips() {
    let code = r"
        def toml = import('ext.formats.toml')
        def text = 'when = 1979-05-27\n\n[t]\na = [1, 2]\n'
        print(toml.encode(toml.decode(text)) == text)
    ";
    assert_eq!(eval(code), "true\n");
}

#[test]
fn formats_msgpack_round_trips() {
    let code = r"
        def msgpack = import('ext.formats.msgpack')
        # {a: x'00', b: the Uint64 18446744073709551615}
        def data = x'82a161c40100a162cfffffffffffffffff'
        print(msgpack.encode(msgpack.decode(data)) == data)
    ";
    assert_eq!(eval(code), "true\n");
}

#[test]
fn formats_cbor_round_trips() {
    let code = r"
        def cbor = import('ext.formats.cbor')
        # {a: x'00', b: the Uint64 18446744073709551615}
        def data = x'a26161410061621bffffffffffffffff'
        print(cbor.encode(cbor.decode(data)) == data)
    ";
    assert_eq!(eval(code), "true\n");
}

#[test]
fn formats_yaml_round_trips() {
    let code = r#"
        def yaml = import('ext.formats.yaml')
        def text = 'a:\n- 1\n- "true"\nb: 1.0\n'
        print(yaml.encode(yaml.decode(text)) == text)
    "#;
    assert_eq!(eval(code), "true\n");
}

/// `frost` links the bundled SQLite, whose header the bindings were built
/// from.
#[test]
fn sqlite_is_installed_bundled() {
    let code = r"
        def sqlite = import('ext.sqlite')
        print(sqlite.version)
    ";
    let bundled = frostlang_sqlite::rusqlite::ffi::SQLITE_VERSION
        .to_str()
        .expect("the version is UTF-8");
    assert_eq!(eval(code), format!("{bundled}\n"));
}
