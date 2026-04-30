use super::{query, ProbeError};
use std::io::{Read, Write};
use std::time::Duration;

/// Tiny query: send a 1x1 RGBA-style "query" image.
/// (`a=q` = action: query — terminal answers without rendering anything.)
const KITTY_QUERY: &[u8] = b"\x1b_Gi=31,a=q,t=d,f=24,s=1,v=1;AAAA\x1b\\";

const ESC: u8 = b'\x1b';

/// Probe the terminal for Kitty graphics protocol support.
/// Timeouts are normal (most terminals don't speak Kitty); they're returned
/// as `Ok(false)` so the doctor pipeline can keep going.
pub fn probe_kitty<W: Write, R: Read>(
    writer: &mut W,
    reader: &mut R,
    timeout: Duration,
) -> Result<bool, ProbeError> {
    match query(writer, reader, KITTY_QUERY, ESC, timeout) {
        Ok(response) => Ok(parse_response_supports_kitty(&response)),
        Err(ProbeError::Timeout) => Ok(false),
        Err(e) => Err(e),
    }
}

pub fn parse_response_supports_kitty(response: &[u8]) -> bool {
    let s = match std::str::from_utf8(response) {
        Ok(s) => s,
        Err(_) => return false,
    };
    s.contains(";OK")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::time::Duration;

    #[test]
    fn parses_ok_response() {
        // "\x1b_Gi=31;OK\x1b" (the trailing \x1b would have been the terminator).
        assert!(parse_response_supports_kitty(b"_Gi=31;OK"));
    }

    #[test]
    fn parses_error_response() {
        assert!(!parse_response_supports_kitty(
            b"_Gi=31;ENOTSUPPORTED:not implemented"
        ));
    }

    #[test]
    fn parses_unrelated_response() {
        // Some terminals may echo back the query bytes — that's not a Kitty OK.
        assert!(!parse_response_supports_kitty(b"\x1b[c"));
    }

    #[test]
    fn full_query_round_trip() {
        let mut writer: Vec<u8> = Vec::new();
        let mut reader = Cursor::new(b"_Gi=31;OK\x1b".to_vec());
        let supports = probe_kitty(&mut writer, &mut reader, Duration::from_millis(50)).unwrap();
        assert!(writer.starts_with(b"\x1b_Gi=31"));
        assert!(supports);
    }

    #[test]
    fn timeout_returns_false_not_error() {
        // No reader response — should NOT bubble the timeout as a Result::Err;
        // we want a clean "false" so probing doesn't fail-stop the doctor.
        let mut writer: Vec<u8> = Vec::new();
        let mut reader = Cursor::new(Vec::<u8>::new());
        let supports = probe_kitty(&mut writer, &mut reader, Duration::from_millis(20)).unwrap();
        assert!(!supports);
    }
}
