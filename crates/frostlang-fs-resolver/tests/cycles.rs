//! A module that imports itself, however indirectly, is an import cycle: the
//! import that closes it raises an error naming the chain.

use crate::common;

use common::{Tree, importer, run};
use frostlang::Value;

#[test]
fn a_module_importing_itself_is_a_cycle() {
    let tree = Tree::new("cycles/self");
    tree.file("me.frst", "import('me')");
    let importer = importer(tree.resolver(&["."]));
    assert_eq!(
        run("import('me')", &importer).message(),
        "Import cycle: me -> me"
    );
}

#[test]
fn two_modules_importing_each_other_are_a_cycle() {
    let tree = Tree::new("cycles/pair");
    tree.file("a.frst", "import('b')")
        .file("b.frst", "import('a')");
    let importer = importer(tree.resolver(&["."]));
    assert_eq!(
        run("import('a')", &importer).message(),
        "Import cycle: a -> b -> a"
    );
}

#[test]
fn the_cycle_names_only_its_own_chain() {
    // `entry` leads into the triangle but is not part of it.
    let tree = Tree::new("cycles/triangle");
    tree.file("entry.frst", "import('a')")
        .file("a.frst", "import('b')")
        .file("b.frst", "import('c')")
        .file("c.frst", "import('a')");
    let importer = importer(tree.resolver(&["."]));
    assert_eq!(
        run("import('entry')", &importer).message(),
        "Import cycle: a -> b -> c -> a"
    );
}

#[test]
fn a_cycle_through_another_specification_of_the_same_file_is_a_cycle() {
    // With overlapping roots, `pkg.a` and `a` are one file.
    let tree = Tree::new("cycles/two_specifications");
    tree.file("pkg/a.frst", "import('b')")
        .file("pkg/b.frst", "import('pkg.a')");
    let importer = importer(tree.resolver(&[".", "pkg"]));
    assert_eq!(
        run("import('a')", &importer).message(),
        "Import cycle: a -> b -> pkg.a"
    );
}

#[test]
fn a_cycle_is_detected_without_caching() {
    let tree = Tree::new("cycles/uncached");
    tree.file("a.frst", "import('b')")
        .file("b.frst", "import('a')");
    let importer = importer(tree.resolver(&["."]).with_caching(false));
    assert_eq!(
        run("import('a')", &importer).message(),
        "Import cycle: a -> b -> a"
    );
}

#[test]
fn a_diamond_is_not_a_cycle() {
    let tree = Tree::new("cycles/diamond");
    tree.file(
        "top.frst",
        "export def both = [import('left').x, import('right').x]",
    )
    .file("left.frst", "export def x = import('base').x")
    .file("right.frst", "export def x = import('base').x")
    .file("base.frst", "export def x = 1");
    for caching in [true, false] {
        let importer = importer(tree.resolver(&["."]).with_caching(caching));
        assert_eq!(
            run("import('top').both", &importer).value(),
            Value::array([1, 1]),
            "caching: {caching}"
        );
    }
}

#[test]
fn a_caught_cycle_leaves_later_imports_unaffected() {
    // `a` catches the cycle it closes, so each load ends normally; a later chain
    // must see none of the earlier loads still in progress.
    let tree = Tree::new("cycles/caught");
    tree.file("a.frst", "export def inner = try_call(import, ['b']).error")
        .file("b.frst", "import('a')")
        .file("c.frst", "import('d')")
        .file("d.frst", "import('c')");
    let importer = importer(tree.resolver(&["."]));
    let value = run(
        r"
        def caught = import('a').inner
        def later = try_call(import, ['c']).error
        [caught, later]
        ",
        &importer,
    )
    .value();
    assert_eq!(
        value,
        Value::array(["Import cycle: a -> b -> a", "Import cycle: c -> d -> c"])
    );
}

#[test]
fn a_failed_load_leaves_no_trace_in_a_later_cycle() {
    // `broken` fails while `a` is loading it; the later cycle names only its own
    // chain.
    let tree = Tree::new("cycles/after_failure");
    tree.file(
        "a.frst",
        r"
        def failed = try_call(import, ['broken']).ok
        import('b')
        ",
    )
    .file("broken.frst", "error('boom')")
    .file("b.frst", "import('a')");
    let importer = importer(tree.resolver(&["."]));
    assert_eq!(
        run("import('a')", &importer).message(),
        "Import cycle: a -> b -> a"
    );
}
