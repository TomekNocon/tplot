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
fn sparkline_whitespace_input_snapshot() {
    let out = run_with_stdin(&["spark", "-"], "1 3 2 5 4 7 9 8 10 6");
    insta::assert_snapshot!("sparkline_whitespace", strip_ansi(&out));
}

#[test]
fn sparkline_csv_column_snapshot() {
    let csv = "t,v\n1,10\n2,30\n3,80\n4,200\n5,150\n6,90\n";
    let out = run_with_stdin(&["spark", "-", "-y", "v"], csv);
    insta::assert_snapshot!("sparkline_csv_column", strip_ansi(&out));
}

#[test]
fn sparkline_no_color_snapshot() {
    let out = run_with_stdin(&["spark", "-", "--no-color"], "1 3 2 5 4 7");
    insta::assert_snapshot!("sparkline_no_color", strip_ansi(&out));
}
