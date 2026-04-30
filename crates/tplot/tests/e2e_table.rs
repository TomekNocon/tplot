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

fn run(args: &[&str]) -> String {
    let out = Command::new(binary_path())
        .args(args)
        .current_dir(workspace_root())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("tplot binary failed");
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
fn table_with_bars_and_sort_snapshot() {
    let out = run(&[
        "table",
        "tests/fixtures/sales.csv",
        "--bars",
        "revenue",
        "--sort",
        "revenue",
        "--width",
        "70",
    ]);
    insta::assert_snapshot!("table_with_bars_and_sort", strip_ansi(&out));
}

#[test]
fn table_top_n_snapshot() {
    let out = run(&[
        "table",
        "tests/fixtures/sales.csv",
        "--sort",
        "revenue",
        "--top",
        "5",
        "--width",
        "70",
    ]);
    insta::assert_snapshot!("table_top_n", strip_ansi(&out));
}
