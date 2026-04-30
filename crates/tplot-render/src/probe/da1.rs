use super::{query, ProbeError};
use std::io::{Read, Write};
use std::time::Duration;

const DA1_QUERY: &[u8] = b"\x1b[c";
const DA1_TERMINATOR: u8 = b'c';

/// Probe the terminal for Sixel graphics support via DA1.
pub fn probe_sixel<W: Write, R: Read>(
    writer: &mut W,
    reader: &mut R,
    timeout: Duration,
) -> Result<bool, ProbeError> {
    let response = query(writer, reader, DA1_QUERY, DA1_TERMINATOR, timeout)?;
    Ok(parse_response_supports_sixel(&response))
}

/// Parse a DA1 response and return true if "4" appears as a standalone
/// numeric feature code. Response format: "\x1b[?<ps>;<ps>;...c", e.g.
/// "\x1b[?62;4;22c". The leading "\x1b[?" prefix and trailing "c" are
/// optional in our caller's view (read_until strips the terminator).
pub fn parse_response_supports_sixel(response: &[u8]) -> bool {
    // Find the last "?" — features come after it, separated by ";".
    let s = match std::str::from_utf8(response) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let after_question = match s.rfind('?') {
        Some(i) => &s[i + 1..],
        None => s,
    };
    after_question.split(';').any(|f| f.trim() == "4")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::time::Duration;

    #[test]
    fn parses_sixel_capable_response() {
        // Real iTerm2 / xterm-with-sixel response: "\x1b[?62;4;22c"
        let response = b"\x1b[?62;4;22";
        assert!(parse_response_supports_sixel(response));
    }

    #[test]
    fn parses_sixel_absent_response() {
        // A response without "4" (no sixel)
        let response = b"\x1b[?62;22";
        assert!(!parse_response_supports_sixel(response));
    }

    #[test]
    fn parses_response_with_4_only_in_a_substring() {
        // "?64;1" should NOT match because "4" appears only as part of "64".
        let response = b"\x1b[?64;1";
        assert!(!parse_response_supports_sixel(response));
    }

    #[test]
    fn full_query_round_trip() {
        let mut writer: Vec<u8> = Vec::new();
        let mut reader = Cursor::new(b"\x1b[?62;4;22c".to_vec());
        let supports = probe_sixel(&mut writer, &mut reader, Duration::from_millis(50)).unwrap();
        assert_eq!(writer, b"\x1b[c");
        assert!(supports);
    }
}
