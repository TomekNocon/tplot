//! Kitty graphics protocol encoder with chunked base64.

use base64::{Engine, engine::general_purpose};

const CHUNK_LEN: usize = 4096;

/// Encode `png_bytes` for the Kitty graphics protocol (transmit + display,
/// PNG format, chunked).
pub fn encode_kitty(png_bytes: &[u8]) -> Vec<u8> {
    let b64 = general_purpose::STANDARD.encode(png_bytes);
    let bytes = b64.as_bytes();
    let total = bytes.len();
    let mut out = Vec::with_capacity(total + 256);

    if total <= CHUNK_LEN {
        // Single APC.
        out.extend_from_slice(b"\x1b_Ga=T,f=100,m=0;");
        out.extend_from_slice(bytes);
        out.extend_from_slice(b"\x1b\\");
    } else {
        // First chunk includes the action header + m=1 (more follows).
        let first_end = CHUNK_LEN;
        out.extend_from_slice(b"\x1b_Ga=T,f=100,m=1;");
        out.extend_from_slice(&bytes[..first_end]);
        out.extend_from_slice(b"\x1b\\");

        let mut start = first_end;
        while start < total {
            let end = (start + CHUNK_LEN).min(total);
            let is_last = end == total;
            let m_flag = if is_last { "0" } else { "1" };
            out.extend_from_slice(format!("\x1b_Gm={m_flag};").as_bytes());
            out.extend_from_slice(&bytes[start..end]);
            out.extend_from_slice(b"\x1b\\");
            start = end;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_payload_is_a_single_apc() {
        let png = vec![0x89, b'P', b'N', b'G']; // tiny PNG
        let out = encode_kitty(&png);
        let s = String::from_utf8_lossy(&out);
        // Should contain exactly ONE APC pair: ESC _ G ... ESC \
        let starts = s.matches("\x1b_G").count();
        let ends = s.matches("\x1b\\").count();
        assert_eq!(starts, 1);
        assert_eq!(ends, 1);
        // First APC has the transmit-and-display action.
        assert!(s.contains("a=T"));
        assert!(s.contains("f=100"));
        // Single-chunk payload uses m=0 (no further chunks).
        assert!(s.contains("m=0;"));
    }

    #[test]
    fn large_payload_splits_into_multiple_chunks() {
        // Use a payload that base64-encodes to >> 4096 chars.
        let png = vec![0xab; 8 * 1024]; // 8 KB raw → ~10920 chars base64 → 3 chunks
        let out = encode_kitty(&png);
        let s = String::from_utf8_lossy(&out);
        let chunk_count = s.matches("\x1b_G").count();
        assert!(chunk_count >= 3, "expected ≥3 chunks, got {chunk_count}");
        // First chunk has a=T, intermediates have only m=1, last has m=0.
        assert!(s.matches("a=T").count() == 1);
        assert!(s.matches("m=1;").count() >= 2);
        assert!(s.matches("m=0;").count() == 1);
    }
}
