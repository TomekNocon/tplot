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
fn candlestick_8_days_snapshot() {
    let csv = "date,o,h,l,c\n\
        d1,100,112,98,110\nd2,110,113,99,105\nd3,105,118,104,115\nd4,115,117,113,115\n\
        d5,115,121,110,118\nd6,118,119,114,116\nd7,116,125,116,124\nd8,124,127,120,121\n";
    let out = run_with_stdin(
        &[
            "candle", "-", "-x", "date", "--open", "o", "--high", "h", "--low", "l", "--close",
            "c", "--width", "80",
        ],
        csv,
    );
    insta::assert_snapshot!("candlestick_8_days", strip_ansi(&out));
}
