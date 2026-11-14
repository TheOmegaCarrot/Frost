//! Frost itself never depends on Tokio: only hosts and extensions that use
//! this crate do.

use std::process::Command;

/// The packages `frostlang` depends on for building, with the features
/// `features` (Cargo's arguments selecting them), one name per line.
fn frostlang_dependencies(features: &[&str]) -> String {
    let cargo = std::env::var("CARGO").expect("Cargo runs the tests");
    let output = Command::new(cargo)
        .args(["tree", "--frozen", "--package", "frostlang"])
        .args([
            "--edges",
            "normal,build",
            "--prefix",
            "none",
            "--format",
            "{p}",
        ])
        .args(features)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("cargo tree runs");
    assert!(
        output.status.success(),
        "cargo tree failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("cargo tree writes UTF-8")
}

#[test]
fn frostlang_pulls_in_no_tokio_with_any_features() {
    for features in [&["--no-default-features"][..], &[], &["--all-features"]] {
        let dependencies = frostlang_dependencies(features);
        assert!(
            dependencies
                .lines()
                .any(|line| line.starts_with("frostlang v")),
            "the tree lists frostlang itself ({features:?}):\n{dependencies}"
        );
        let tokio: Vec<&str> = dependencies
            .lines()
            .filter(|line| line.starts_with("tokio"))
            .collect();
        assert!(
            tokio.is_empty(),
            "frostlang must not depend on Tokio ({features:?}), but pulls in {tokio:?}"
        );
    }
}
