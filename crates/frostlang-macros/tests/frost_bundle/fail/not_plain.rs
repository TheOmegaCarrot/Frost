// A plain inherent impl block of plain functions: not a trait impl, not
// generic, and no `async` or generic function.

use frostlang::frost_bundle;

trait Named {
    fn name(&self) -> String;
}

struct Thing;

#[frost_bundle(prefix = "thing")]
impl Named for Thing {
    fn name(&self) -> String {
        String::from("thing")
    }
}

struct Wrapper<T>(T);

#[frost_bundle(prefix = "wrapper")]
impl<T: Send + Sync> Wrapper<T> {}

struct Plain;

#[frost_bundle(prefix = "plain")]
impl Plain {
    async fn later(&self) {}

    fn generic<T>(&self) {}
}

fn main() {}
