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
fn boxplot_three_groups_snapshot() {
    let csv = "endpoint,ms\n\
        /users,48\n/users,50\n/users,51\n/users,52\n/users,53\n\
        /orders,30\n/orders,60\n/orders,100\n/orders,250\n/orders,400\n/orders,80\n\
        /health,5\n/health,6\n/health,7\n/health,8\n/health,9\n";
    let out = run_with_stdin(
        &["box", "-", "-x", "endpoint", "-y", "ms", "--width", "80"],
        csv,
    );
    insta::assert_snapshot!("boxplot_three_groups", strip_ansi(&out));
}

#[test]
fn boxplot_neutral_snapshot() {
    let csv = "endpoint,ms\n\
        /users,48\n/users,50\n/users,51\n/users,52\n/users,53\n\
        /orders,30\n/orders,60\n/orders,100\n/orders,250\n/orders,400\n/orders,80\n";
    let out = run_with_stdin(
        &[
            "box",
            "-",
            "-x",
            "endpoint",
            "-y",
            "ms",
            "--neutral",
            "--width",
            "80",
        ],
        csv,
    );
    insta::assert_snapshot!("boxplot_neutral", strip_ansi(&out));
}
