use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn binary_path() -> &'static str {
    env!("CARGO_BIN_EXE_tplot")
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR points at crates/tplot at compile time; ../.. is the workspace root.
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root above crates/tplot")
        .to_path_buf()
}

fn run(args: &[&str]) -> String {
    let out = Command::new(binary_path())
        .args(args)
        .current_dir(workspace_root())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("tplot binary failed to launch");
    String::from_utf8(out.stdout).expect("non-utf8 stdout")
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
    // Minimal ANSI strip — good enough for snapshot diff readability.
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
fn vertical_bar_default_snapshot() {
    let out = run(&[
        "bar",
        "tests/fixtures/sales.csv",
        "-x",
        "quarter",
        "-y",
        "revenue",
        "--group",
        "region",
        "--vertical",
        "--width",
        "80",
    ]);
    insta::assert_snapshot!("vertical_bar_default", strip_ansi(&out));
}

#[test]
fn histogram_default_snapshot() {
    let csv = "ms\n10\n22\n35\n41\n48\n49\n50\n50\n51\n52\n55\n58\n60\n65\n80\n95\n110\n145\n220\n";
    let out = run_with_stdin(
        &["hist", "-", "-x", "ms", "--bins", "7", "--width", "80"],
        csv,
    );
    insta::assert_snapshot!("histogram_default", strip_ansi(&out));
}

#[test]
fn histogram_neutral_snapshot() {
    let csv = "ms\n10\n22\n35\n41\n48\n49\n50\n50\n51\n52\n55\n58\n60\n65\n80\n95\n110\n145\n220\n";
    let out = run_with_stdin(
        &[
            "hist",
            "-",
            "-x",
            "ms",
            "--bins",
            "7",
            "--neutral",
            "--width",
            "80",
        ],
        csv,
    );
    insta::assert_snapshot!("histogram_neutral", strip_ansi(&out));
}
