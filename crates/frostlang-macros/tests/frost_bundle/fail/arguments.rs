// `prefix` is required; `group` is optional; each is given once, and there
// are no others.

use frostlang::frost_bundle;

struct Missing;

#[frost_bundle]
impl Missing {}

struct Unknown;

#[frost_bundle(prefix = "unknown", strict = true)]
impl Unknown {}

struct Twice;

#[frost_bundle(prefix = "twice", prefix = "again")]
impl Twice {}

fn main() {}
