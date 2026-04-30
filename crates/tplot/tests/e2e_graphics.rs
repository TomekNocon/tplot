//! End-to-end tests for the `--graphics` flag — we shell out to the binary
//! and check that the stdout starts with the expected escape sequence prefix
//! (or, for `none`, that it does NOT carry an iTerm2/Kitty prefix).
//!
//! We don't snapshot the bytes because the PNG payload depends on the `image`
//! crate's encoder version and would be brittle.
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

fn run(args: &[&str]) -> Vec<u8> {
    Command::new(binary_path())
        .args(args)
        .current_dir(workspace_root())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("tplot binary failed to launch")
        .stdout
}

#[test]
fn iterm2_graphics_starts_with_correct_osc() {
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
        "60",
        "--graphics",
        "iterm2",
    ]);
    assert!(
        out.starts_with(b"\x1b]1337;File="),
        "expected iTerm2 OSC prefix; got: {:?}",
        &out[..out.len().min(40)]
    );
}

#[test]
fn kitty_graphics_starts_with_correct_apc() {
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
        "60",
        "--graphics",
        "kitty",
    ]);
    assert!(
        out.starts_with(b"\x1b_Ga=T"),
        "expected Kitty APC prefix; got: {:?}",
        &out[..out.len().min(40)]
    );
}

#[test]
fn graphics_none_produces_text_rendering() {
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
        "60",
        // no --graphics flag → defaults to "none" → text path.
    ]);
    let s = String::from_utf8_lossy(&out);
    // Text path emits ANSI color escapes, NOT the iTerm2/Kitty image escapes.
    assert!(!s.starts_with("\x1b]1337"));
    assert!(!s.starts_with("\x1b_G"));
    // EMEA label should appear (story-pass focal under default sales fixture).
    assert!(
        s.contains("EMEA"),
        "expected EMEA label in text output; got: {s}"
    );
}
