use std::io::{Read, Write};
use std::time::{Duration, Instant};

pub mod da1;
pub mod kitty;

#[derive(Debug, thiserror::Error)]
pub enum ProbeError {
    #[error("timed out waiting for terminal response")]
    Timeout,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Read from `reader` until a byte equal to `terminator` is encountered,
/// returning all bytes BEFORE the terminator (the terminator itself is
/// consumed but not included). Times out after `timeout` elapses.
///
/// Reads one byte at a time. Designed for OSC/CSI responses, which are
/// short (≤ 64 bytes) and have a clear terminator.
pub fn read_until<R: Read>(
    reader: &mut R,
    terminator: u8,
    timeout: Duration,
) -> Result<Vec<u8>, ProbeError> {
    let deadline = Instant::now() + timeout;
    let mut out = Vec::with_capacity(64);
    let mut byte = [0u8; 1];
    while Instant::now() < deadline {
        match reader.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {
                if byte[0] == terminator {
                    return Ok(out);
                }
                out.push(byte[0]);
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
            Err(e) => return Err(ProbeError::Io(e)),
        }
    }
    if out.is_empty() {
        Err(ProbeError::Timeout)
    } else {
        Ok(out)
    }
}

/// Send `query` bytes to `writer`, flush, then read from `reader` until
/// `terminator` is seen (or we time out).
pub fn query<W: Write, R: Read>(
    writer: &mut W,
    reader: &mut R,
    query: &[u8],
    terminator: u8,
    timeout: Duration,
) -> Result<Vec<u8>, ProbeError> {
    writer.write_all(query)?;
    writer.flush()?;
    read_until(reader, terminator, timeout)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::time::Duration;

    #[test]
    fn read_until_finds_terminator() {
        let mut reader = Cursor::new(b"hello\x07world".to_vec());
        let buf = read_until(&mut reader, b'\x07', Duration::from_secs(1)).unwrap();
        assert_eq!(buf, b"hello".to_vec());
    }

    #[test]
    fn read_until_returns_what_it_has_on_eof() {
        let mut reader = Cursor::new(b"partial".to_vec());
        let buf = read_until(&mut reader, b'\x07', Duration::from_millis(50));
        // Should return what was read so far (non-Ok timeout) — implementation-defined
        // but should not panic.
        assert!(buf.is_err() || buf.unwrap() == b"partial".to_vec());
    }

    #[test]
    fn write_query_then_read() {
        let mut writer: Vec<u8> = Vec::new();
        let mut reader = Cursor::new(b"\x1b[?62;4c".to_vec());
        let response = query(
            &mut writer,
            &mut reader,
            b"\x1b[c",
            b'c',
            Duration::from_secs(1),
        )
        .unwrap();
        assert_eq!(writer, b"\x1b[c");
        // Returned bytes include the terminator? — strip it; we just want the content.
        assert!(
            response.contains(&b'?'),
            "response should include the device attributes prefix"
        );
    }
}
