//! The extension's shape: what `import('ext.datetime')` holds.

use crate::common::assert_values;

#[test]
fn the_extension_holds_its_functions() {
    assert_values(&[(
        "sorted(keys(dt))",
        "['before', 'components', 'format', 'format_duration', 'instant', 'local_zone', \
         'parse', 'parse_duration', 'to_rfc3339', 'to_rfc9557', 'to_string', 'weekday']",
    )]);
}
