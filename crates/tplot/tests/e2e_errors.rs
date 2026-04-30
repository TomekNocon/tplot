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

fn run_expecting_error(args: &[&str]) -> String {
    let out = Command::new(binary_path())
        .args(args)
        .current_dir(workspace_root())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("tplot binary failed to launch");
    String::from_utf8(out.stderr).expect("non-utf8 stderr")
}

#[test]
fn typo_column_lists_did_you_mean() {
    let stderr = run_expecting_error(&[
        "bar",
        "tests/fixtures/sales.csv",
        "-x",
        "regin",
        "-y",
        "revenue",
    ]);
    insta::assert_snapshot!("error_typo_column", stderr);
}

#[test]
fn non_numeric_y_lists_alternatives() {
    let stderr = run_expecting_error(&[
        "bar",
        "tests/fixtures/sales.csv",
        "-x",
        "revenue",
        "-y",
        "quarter",
        "--group",
        "region",
    ]);
    insta::assert_snapshot!("error_non_numeric_y", stderr);
}

#[test]
fn narrow_terminal_rejects_below_40() {
    let stderr = run_expecting_error(&[
        "bar",
        "tests/fixtures/sales.csv",
        "-x",
        "quarter",
        "-y",
        "revenue",
        "--group",
        "region",
        "--width",
        "30",
    ]);
    insta::assert_snapshot!("error_narrow_terminal", stderr);
}
