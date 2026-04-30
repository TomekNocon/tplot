use std::io::Write;
use std::process::{Command, Stdio};

fn binary_path() -> &'static str {
    env!("CARGO_BIN_EXE_tplot")
}
fn workspace_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf()
}

fn run_with_stdin(args: &[&str], input: &str) -> String {
    let mut child = Command::new(binary_path())
        .args(args)
        .current_dir(workspace_root())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("tplot binary failed");
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let out = child.wait_with_output().expect("wait failed");
    String::from_utf8(out.stdout).expect("non-utf8 stdout")
}

fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' && chars.peek() == Some(&'[') {
            chars.next();
            for c2 in chars.by_ref() {
                if c2.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[test]
fn summary_sequence_mode_snapshot() {
    let csv = "v\n1\n3\n2\n5\n4\n7\n9\n8\n10\n6\n";
    let out = run_with_stdin(&["summary", "-", "-y", "v"], csv);
    insta::assert_snapshot!("summary_sequence", strip_ansi(&out));
    // Single-line guarantee.
    assert_eq!(
        out.matches('\n').count(),
        1,
        "expected exactly 1 newline: {out:?}"
    );
}

#[test]
fn summary_categorical_mode_snapshot() {
    let csv = "lang,lines\nrust,1832\nmarkdown,892\nshell,541\nyaml,128\ntoml,89\n";
    let out = run_with_stdin(&["summary", "-", "-x", "lang", "-y", "lines"], csv);
    insta::assert_snapshot!("summary_categorical", strip_ansi(&out));
    assert_eq!(out.matches('\n').count(), 1);
}
