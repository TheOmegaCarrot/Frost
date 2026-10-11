// A free function only: methods over shared state belong in a bundle.

use frostlang::frost_fn;

struct Counter(i64);

impl Counter {
    #[frost_fn(name = "counter.get")]
    fn get(&self) -> i64 {
        self.0
    }
}

fn main() {}
