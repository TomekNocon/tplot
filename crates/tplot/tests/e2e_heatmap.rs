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
fn heatmap_inferno_snapshot() {
    let csv = "h,d,c\n9,Mon,5\n10,Mon,12\n11,Mon,8\n9,Tue,3\n10,Tue,25\n11,Tue,7\n";
    let out = run_with_stdin(
        &[
            "heatmap", "-", "-x", "h", "-y", "d", "--value", "c", "--ramp", "inferno", "--width",
            "60",
        ],
        csv,
    );
    insta::assert_snapshot!("heatmap_inferno", strip_ansi(&out));
}

#[test]
fn heatmap_viridis_snapshot() {
    let csv = "h,d,c\n9,Mon,5\n10,Mon,12\n11,Mon,8\n9,Tue,3\n10,Tue,25\n11,Tue,7\n";
    let out = run_with_stdin(
        &[
            "heatmap", "-", "-x", "h", "-y", "d", "--value", "c", "--ramp", "viridis", "--width",
            "60",
        ],
        csv,
    );
    insta::assert_snapshot!("heatmap_viridis", strip_ansi(&out));
}
