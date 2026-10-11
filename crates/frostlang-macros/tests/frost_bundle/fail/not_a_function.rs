// Every item in the block is one of the group's functions.

use frostlang::frost_bundle;

struct Constant;

#[frost_bundle(prefix = "constant")]
impl Constant {
    const LIMIT: i64 = 10;

    fn limit(&self) -> i64 {
        Self::LIMIT
    }
}

fn main() {}
