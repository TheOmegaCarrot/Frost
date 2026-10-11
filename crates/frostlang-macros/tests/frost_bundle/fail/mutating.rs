// A bundle over state that never changes cannot change it.

use frostlang::frost_bundle;

struct Counter(i64);

#[frost_bundle(prefix = "counter")]
impl Counter {
    fn add(&mut self, amount: i64) -> i64 {
        self.0 += amount;
        self.0
    }
}

fn main() {}
