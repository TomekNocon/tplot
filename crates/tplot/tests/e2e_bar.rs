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
        .output()
        .expect("tplot binary failed to launch");
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
fn bar_default_story_pass_snapshot() {
    let out = run(&[
        "bar",
        "tests/fixtures/sales.csv",
        "-x",
        "quarter",
        "-y",
        "revenue",
        "--group",
        "region",
        "--width",
        "80",
    ]);
    insta::assert_snapshot!("bar_default", strip_ansi(&out));
}

#[test]
fn bar_neutral_snapshot() {
    let out = run(&[
        "bar",
        "tests/fixtures/sales.csv",
        "-x",
        "quarter",
        "-y",
        "revenue",
        "--group",
        "region",
        "--neutral",
        "--width",
        "80",
    ]);
    insta::assert_snapshot!("bar_neutral", strip_ansi(&out));
}

#[test]
fn bar_user_focus_snapshot() {
    let out = run(&[
        "bar",
        "tests/fixtures/sales.csv",
        "-x",
        "quarter",
        "-y",
        "revenue",
        "--group",
        "region",
        "--focus",
        "NA",
        "--width",
        "80",
    ]);
    insta::assert_snapshot!("bar_focus_na", strip_ansi(&out));
}
