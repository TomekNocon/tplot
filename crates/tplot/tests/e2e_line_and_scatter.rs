use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn binary_path() -> &'static str {
    env!("CARGO_BIN_EXE_tplot")
}

fn workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root above crates/tplot")
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
        .expect("tplot binary failed to launch");
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
fn line_multi_series_snapshot() {
    let csv = "t,v,g\n1,10,A\n2,11,A\n3,9,A\n4,12,A\n1,5,B\n2,30,B\n3,80,B\n4,200,B\n";
    let out = run_with_stdin(
        &[
            "line", "-", "-x", "t", "-y", "v", "--group", "g", "--width", "80",
        ],
        csv,
    );
    insta::assert_snapshot!("line_multi_series", strip_ansi(&out));
}

#[test]
fn scatter_default_snapshot() {
    let csv = "x,y\n1,10\n2,12\n3,15\n4,18\n5,30\n6,28\n7,22\n8,18\n9,15\n10,12\n";
    let out = run_with_stdin(
        &["scatter", "-", "-x", "x", "-y", "y", "--width", "80"],
        csv,
    );
    insta::assert_snapshot!("scatter_default", strip_ansi(&out));
}
