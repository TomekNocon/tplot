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
fn violin_two_groups_snapshot() {
    let csv = "endpoint,ms\n\
        /short,48\n/short,50\n/short,51\n/short,52\n/short,53\n\
        /long,10\n/long,30\n/long,80\n/long,150\n/long,300\n/long,250\n/long,60\n/long,90\n";
    let out = run_with_stdin(
        &["violin", "-", "-x", "endpoint", "-y", "ms", "--width", "70"],
        csv,
    );
    insta::assert_snapshot!("violin_two_groups", strip_ansi(&out));
}
