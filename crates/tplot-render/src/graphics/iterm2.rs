//! iTerm2 inline-image protocol encoder.

use base64::{Engine, engine::general_purpose};

/// Encode `png_bytes` as an iTerm2 inline-image OSC sequence.
/// Format:
///   ESC ] 1337 ; File = inline=1 ; size=<n> : <base64> BEL
pub fn encode_iterm2(png_bytes: &[u8]) -> Vec<u8> {
    let b64 = general_purpose::STANDARD.encode(png_bytes);
    let header = format!("\x1b]1337;File=inline=1;size={}:", png_bytes.len());
    let mut out = Vec::with_capacity(header.len() + b64.len() + 1);
    out.extend_from_slice(header.as_bytes());
    out.extend_from_slice(b64.as_bytes());
    out.push(0x07); // BEL
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_starts_with_iterm2_osc_prefix() {
        let png = vec![0x89, b'P', b'N', b'G', 0, 0, 0, 0]; // fake PNG-ish bytes
        let out = encode_iterm2(&png);
        assert!(out.starts_with(b"\x1b]1337;File="));
    }

    #[test]
    fn output_includes_size_and_inline_flag() {
        let png = vec![0; 100];
        let out = encode_iterm2(&png);
        let s = String::from_utf8_lossy(&out);
        assert!(s.contains("size=100"));
        assert!(s.contains("inline=1"));
    }

    #[test]
    fn output_is_base64_terminated_by_bell() {
        let png = b"hello world".to_vec();
        let out = encode_iterm2(&png);
        assert_eq!(*out.last().unwrap(), 0x07);
    }

    #[test]
    fn base64_payload_round_trips() {
        use base64::{Engine, engine::general_purpose};
        let original = b"\x89PNGtest\xc3\x9d";
        let out = encode_iterm2(original);
        let s = String::from_utf8_lossy(&out);
        // Extract the part between ":" and the trailing BEL
        let payload = s.split(':').nth(1).unwrap().trim_end_matches('\x07');
        let decoded = general_purpose::STANDARD.decode(payload).unwrap();
        assert_eq!(decoded, original);
    }
}
