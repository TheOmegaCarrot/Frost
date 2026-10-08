//! `frost repl` run as a process, for what depends on its standard streams.

use std::io::Write;
use std::process::{Command, Output, Stdio};

/// What `frost repl` writes, given `input` on a pipe for standard input.
fn repl_on_piped_input(input: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_frost"))
        .arg("repl")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("`frost` should start");
    child
        .stdin
        .take()
        .expect("standard input is piped")
        .write_all(input.as_bytes())
        .expect("`frost` should take its input");
    child.wait_with_output().expect("`frost` should finish")
}

#[test]
fn piped_input_gets_no_prompts() {
    let input = r"1 + 1
def x = 1
defn inc(n) ->
    n + 1
print('printed')
inc(x)
";
    let output = repl_on_piped_input(input);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "2\nprinted\n2\n");
    assert_eq!(String::from_utf8(output.stderr).unwrap(), "");
}

#[test]
fn piped_input_gets_failures_on_standard_error_only() {
    let output = repl_on_piped_input("nope\n3\n");
    assert!(output.status.success(), "{output:?}");
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "3\n");
    let errors = String::from_utf8(output.stderr).unwrap();
    assert!(errors.contains("`nope` is not defined"), "{errors}");
}
