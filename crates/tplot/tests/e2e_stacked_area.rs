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
fn stacked_area_three_series_snapshot() {
    let csv = "month,rev,region\n\
        1,10,NA\n1,5,EMEA\n1,3,APAC\n\
        2,15,NA\n2,7,EMEA\n2,5,APAC\n\
        3,30,NA\n3,9,EMEA\n3,6,APAC\n\
        4,40,NA\n4,12,EMEA\n4,8,APAC\n\
        5,55,NA\n5,15,EMEA\n5,10,APAC\n\
        6,70,NA\n6,18,EMEA\n6,11,APAC\n";
    let out = run_with_stdin(
        &[
            "area", "-", "-x", "month", "-y", "rev", "--group", "region", "--width", "80",
        ],
        csv,
    );
    insta::assert_snapshot!("stacked_area_three_series", strip_ansi(&out));
}

#[test]
fn stacked_area_neutral_snapshot() {
    let csv = "month,rev,region\n\
        1,10,NA\n1,5,EMEA\n\
        2,20,NA\n2,5,EMEA\n\
        3,40,NA\n3,6,EMEA\n";
    let out = run_with_stdin(
        &[
            "area",
            "-",
            "-x",
            "month",
            "-y",
            "rev",
            "--group",
            "region",
            "--neutral",
            "--width",
            "80",
        ],
        csv,
    );
    insta::assert_snapshot!("stacked_area_neutral", strip_ansi(&out));
}
