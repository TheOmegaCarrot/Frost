// The macro must name each parameter's type, so nothing generic: by type
// parameter, lifetime, or where clause.

use frostlang::frost_fn;

#[frost_fn(name = "by_type")]
fn by_type<T>(_value: T) {}

#[frost_fn(name = "by_lifetime")]
fn by_lifetime<'a>() {}

#[frost_fn(name = "by_where_clause")]
fn by_where_clause()
where
    i64: Copy,
{
}

fn main() {}
