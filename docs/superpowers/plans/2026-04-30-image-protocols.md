# TerminalPlot — Plan 7b: Image Protocol Output Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task.

**Goal:** Add `--graphics auto|kitty|iterm2|none` to every chart command. When enabled, render the same `PixelBuffer` to a PNG and emit it via the appropriate inline-image protocol so terminals that support graphics (iTerm2, Kitty, WezTerm with Kitty mode) display a pixel-perfect chart instead of half-blocks. `auto` picks the best supported protocol from `Capabilities`; falls back to text rendering if nothing's supported.

**Architecture:** New `graphics` module in `tplot-render` with two encoders. Pipeline: existing `PixelBuffer` → upscale to a sane image size (Plan-1 buffers are tiny, ~80×40 sub-pixels) → `image::DynamicImage` → encode to PNG → wrap in protocol-specific escapes → write to stdout. The `--graphics` flag wires through `CommonStoryArgs` (already in every chart command).

**Tech Stack:** Adds `image` (PNG encoding) and `base64` (or use `data-encoding`) crates. Both are workspace dependencies after Task 1.

**Inherited context (Plans 1–7):**
- 9 chart types, 234 tests, capability probing live, `tplot doctor` reports it.
- `Capabilities { graphics_protocol: GraphicsProtocol }` field already populated by env + probe merge.
- All commits attributed to `tomek.lm10@gmail.com`.

**Sixel deliberately deferred** to Plan 7c (or later) — pure-Rust encoders are immature, and Mac users don't typically encounter sixel-capable terminals.

---

## File Structure

```
crates/
├── tplot-render/src/
│   ├── lib.rs                      EXTEND: pub mod graphics
│   └── graphics/
│       ├── mod.rs                  ★ NEW: PixelBuffer → DynamicImage + dispatch
│       ├── png.rs                  ★ NEW: encode DynamicImage to PNG bytes
│       ├── kitty.rs                ★ NEW: Kitty graphics protocol encoder
│       └── iterm2.rs               ★ NEW: iTerm2 inline-image protocol encoder
├── tplot/src/
│   ├── cli.rs                      EXTEND: --graphics flag value parsing
│   ├── pipeline.rs                 EXTEND: helper that runs graphics path when caps allow
│   └── commands/
│       └── (each command's *.rs)   EXTEND: route to graphics output when --graphics set
└── tplot/tests/
    └── e2e_graphics.rs             ★ NEW: integration tests for protocol output
```

Workspace `Cargo.toml` gets:
```toml
image       = { version = "0.25", default-features = false, features = ["png"] }
base64      = "0.22"
```

---

## Task 1: Add image + base64 dependencies; PixelBuffer → DynamicImage

**Files:**
- Modify: workspace `Cargo.toml`
- Modify: `crates/tplot-render/Cargo.toml`
- Create: `crates/tplot-render/src/graphics/mod.rs`
- Modify: `crates/tplot-render/src/lib.rs`

The `image` crate's `RgbaImage` is the easiest target. Each `PixelBuffer` cell with `Some(RgbColor)` becomes opaque RGBA; `None` becomes transparent.

- [ ] **Step 1: Add deps**

```toml
# workspace Cargo.toml — add to [workspace.dependencies]
image  = { version = "0.25", default-features = false, features = ["png"] }
base64 = "0.22"
```

```toml
# crates/tplot-render/Cargo.toml — add to [dependencies]
image  = { workspace = true }
base64 = { workspace = true }
```

- [ ] **Step 2: Write failing tests**

```rust
// crates/tplot-render/src/graphics/mod.rs
#[cfg(test)]
mod tests {
    use super::*;
    use tplot_core::PixelBuffer;
    use tplot_protocol::RgbColor;

    #[test]
    fn empty_buffer_yields_fully_transparent_image() {
        let buf = PixelBuffer::new(4, 4);
        let img = buffer_to_image(&buf, 1);
        assert_eq!(img.width(), 4);
        assert_eq!(img.height(), 4);
        // Every pixel should have alpha = 0.
        for x in 0..4 {
            for y in 0..4 {
                let pixel = img.get_pixel(x, y);
                assert_eq!(pixel[3], 0, "pixel ({x},{y}) should be transparent");
            }
        }
    }

    #[test]
    fn set_pixel_yields_opaque_rgba() {
        let mut buf = PixelBuffer::new(2, 2);
        let red = RgbColor { r: 255, g: 0, b: 0 };
        buf.set(0, 0, red);
        let img = buffer_to_image(&buf, 1);
        let p = img.get_pixel(0, 0);
        assert_eq!(p[0], 255);
        assert_eq!(p[1], 0);
        assert_eq!(p[2], 0);
        assert_eq!(p[3], 255);
    }

    #[test]
    fn upscale_factor_4_quadruples_dimensions() {
        let buf = PixelBuffer::new(2, 3);
        let img = buffer_to_image(&buf, 4);
        assert_eq!(img.width(), 8);
        assert_eq!(img.height(), 12);
    }
}
```

- [ ] **Step 3: Add module ref**

```rust
// crates/tplot-render/src/lib.rs (append)
pub mod graphics;
```

- [ ] **Step 4: Run (expected fail)**

```bash
cargo test -p tplot-render --lib graphics
```

- [ ] **Step 5: Implement**

```rust
// crates/tplot-render/src/graphics/mod.rs (above tests)
use image::{ImageBuffer, Rgba, RgbaImage};
use tplot_core::PixelBuffer;

pub mod kitty;
pub mod iterm2;
pub mod png;

/// Convert a `PixelBuffer` to an upscaled `RgbaImage`. Each sub-pixel becomes
/// an `upscale × upscale` block so the resulting PNG is large enough that
/// terminals don't render it at micro-size when shown inline.
pub fn buffer_to_image(buf: &PixelBuffer, upscale: u32) -> RgbaImage {
    let upscale = upscale.max(1);
    let pw = buf.pixel_width()  as u32;
    let ph = buf.pixel_height() as u32;
    let out_w = pw * upscale;
    let out_h = ph * upscale;
    let mut img: RgbaImage = ImageBuffer::new(out_w, out_h);

    for src_y in 0..ph {
        for src_x in 0..pw {
            let pixel: Rgba<u8> = match buf.get(src_x as usize, src_y as usize) {
                Some(c) => Rgba([c.r, c.g, c.b, 255]),
                None    => Rgba([0, 0, 0, 0]),
            };
            for dy in 0..upscale {
                for dx in 0..upscale {
                    img.put_pixel(src_x * upscale + dx, src_y * upscale + dy, pixel);
                }
            }
        }
    }
    img
}
```

Empty stub the sibling modules so the crate compiles even before they're filled:

```rust
// crates/tplot-render/src/graphics/kitty.rs
//! Kitty graphics protocol — implemented in Plan 7b Task 4.
```

```rust
// crates/tplot-render/src/graphics/iterm2.rs
//! iTerm2 inline-image protocol — implemented in Plan 7b Task 3.
```

```rust
// crates/tplot-render/src/graphics/png.rs
//! PNG encoding helper — implemented in Plan 7b Task 2.
```

- [ ] **Step 6: Run (expected pass)**

```bash
cargo test -p tplot-render --lib graphics
```

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml crates/tplot-render/
git commit -m "Add image + base64 deps; convert PixelBuffer to RgbaImage"
```

---

## Task 2: PNG encoding helper

**Files:**
- Modify: `crates/tplot-render/src/graphics/png.rs`

Wraps `image::DynamicImage::write_to` with the PNG encoder. Returns the encoded bytes.

- [ ] **Step 1: Write failing tests**

```rust
// crates/tplot-render/src/graphics/png.rs
#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba};

    #[test]
    fn png_starts_with_png_magic_bytes() {
        let img: image::RgbaImage = ImageBuffer::from_pixel(2, 2, Rgba([0xee, 0x7b, 0x3d, 255]));
        let bytes = encode_png(&img).unwrap();
        // PNG magic: \x89 P N G \r \n \x1a \n
        assert_eq!(&bytes[..8], &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]);
    }
}
```

- [ ] **Step 2: Run (expected fail)**

- [ ] **Step 3: Implement**

```rust
// crates/tplot-render/src/graphics/png.rs (above tests)
use image::RgbaImage;
use std::io::Cursor;

/// Encode an `RgbaImage` to PNG bytes.
pub fn encode_png(img: &RgbaImage) -> Result<Vec<u8>, image::ImageError> {
    let mut cursor = Cursor::new(Vec::with_capacity(4096));
    img.write_to(&mut cursor, image::ImageFormat::Png)?;
    Ok(cursor.into_inner())
}
```

- [ ] **Step 4: Run + commit**

```bash
cargo test -p tplot-render --lib graphics::png
git add crates/tplot-render/
git commit -m "Add PNG encoder for graphics output"
```

---

## Task 3: iTerm2 inline-image protocol

**Files:**
- Modify: `crates/tplot-render/src/graphics/iterm2.rs`

iTerm2 protocol is the simplest of the three:

```
ESC ] 1337 ; File = inline=1 ; size=<bytes> : <base64-png> BEL
```

`BEL` is `\x07`. Optional parameters (`width`, `height`, `name`) tweak rendering. We'll keep it minimal.

- [ ] **Step 1: Write failing tests**

```rust
// crates/tplot-render/src/graphics/iterm2.rs
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
        let s = String::from_utf8_lossy(&encode_iterm2(&png));
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
        let payload = s.split(':').nth(1).unwrap()
            .trim_end_matches('\x07');
        let decoded = general_purpose::STANDARD.decode(payload).unwrap();
        assert_eq!(decoded, original);
    }
}
```

- [ ] **Step 2: Run (expected fail)**

- [ ] **Step 3: Implement**

```rust
// crates/tplot-render/src/graphics/iterm2.rs (above tests)
use base64::{Engine, engine::general_purpose};

/// Encode `png_bytes` as an iTerm2 inline-image OSC sequence.
/// Format:
///   ESC ] 1337 ; File = inline=1 ; size=<n> : <base64> BEL
pub fn encode_iterm2(png_bytes: &[u8]) -> Vec<u8> {
    let b64 = general_purpose::STANDARD.encode(png_bytes);
    let header = format!(
        "\x1b]1337;File=inline=1;size={}:",
        png_bytes.len()
    );
    let mut out = Vec::with_capacity(header.len() + b64.len() + 1);
    out.extend_from_slice(header.as_bytes());
    out.extend_from_slice(b64.as_bytes());
    out.push(0x07); // BEL
    out
}
```

- [ ] **Step 4: Run + commit**

```bash
cargo test -p tplot-render --lib graphics::iterm2
git add crates/tplot-render/
git commit -m "Add iTerm2 inline-image protocol encoder"
```

---

## Task 4: Kitty graphics protocol

**Files:**
- Modify: `crates/tplot-render/src/graphics/kitty.rs`

Kitty's protocol uses chunked base64 (one chunk per APC sequence):

```
ESC _ G a=T,f=100,m=1 ; <chunk-1> ESC \
ESC _ G m=1 ; <chunk-2> ESC \
...
ESC _ G m=0 ; <chunk-N> ESC \
```

`a=T` means transmit + display. `f=100` means PNG. Chunk size 4096 bytes max per the spec; 4096 is recommended.

- [ ] **Step 1: Write failing tests**

```rust
// crates/tplot-render/src/graphics/kitty.rs
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
        let ends   = s.matches("\x1b\\").count();
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
```

- [ ] **Step 2: Run (expected fail)**

- [ ] **Step 3: Implement**

```rust
// crates/tplot-render/src/graphics/kitty.rs (above tests)
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
            let end       = (start + CHUNK_LEN).min(total);
            let is_last   = end == total;
            let m_flag    = if is_last { "0" } else { "1" };
            out.extend_from_slice(format!("\x1b_Gm={m_flag};").as_bytes());
            out.extend_from_slice(&bytes[start..end]);
            out.extend_from_slice(b"\x1b\\");
            start = end;
        }
    }
    out
}
```

- [ ] **Step 4: Run + commit**

```bash
cargo test -p tplot-render --lib graphics::kitty
git add crates/tplot-render/
git commit -m "Add Kitty graphics protocol encoder with chunked base64"
```

---

## Task 5: Top-level dispatch — `render_graphics`

**Files:**
- Modify: `crates/tplot-render/src/graphics/mod.rs`

A small wrapper that takes a `PixelBuffer + Capabilities` (or a forced protocol) and produces the appropriate escape-encoded bytes. Falls back to text rendering when graphics isn't supported.

- [ ] **Step 1: Write failing tests**

```rust
// crates/tplot-render/src/graphics/mod.rs (within existing tests module)
#[test]
fn render_graphics_produces_iterm2_when_protocol_is_iterm2() {
    let buf = PixelBuffer::new(4, 4);
    let bytes = render_graphics(&buf, GraphicsProtocol::ITerm2, 4);
    assert!(bytes.starts_with(b"\x1b]1337;File="));
}

#[test]
fn render_graphics_produces_kitty_when_protocol_is_kitty() {
    let buf = PixelBuffer::new(4, 4);
    let bytes = render_graphics(&buf, GraphicsProtocol::Kitty, 4);
    assert!(bytes.starts_with(b"\x1b_Ga=T"));
}

#[test]
fn render_graphics_returns_empty_for_none_or_sixel() {
    // None → empty (caller should fall back to text rendering).
    let buf = PixelBuffer::new(4, 4);
    assert!(render_graphics(&buf, GraphicsProtocol::None,  4).is_empty());
    // Sixel — not implemented in 7b, returns empty (caller falls back).
    assert!(render_graphics(&buf, GraphicsProtocol::Sixel, 4).is_empty());
}
```

- [ ] **Step 2: Run (expected fail)**

- [ ] **Step 3: Implement**

```rust
// crates/tplot-render/src/graphics/mod.rs (append to existing module body)
use tplot_protocol::GraphicsProtocol;

/// Top-level graphics rendering. Returns an empty Vec if the protocol is
/// `None` or unsupported (Sixel, until Plan 7c) — callers should treat that
/// as "fall back to text rendering".
pub fn render_graphics(
    buf:      &PixelBuffer,
    protocol: GraphicsProtocol,
    upscale:  u32,
) -> Vec<u8> {
    match protocol {
        GraphicsProtocol::None | GraphicsProtocol::Sixel => Vec::new(),
        GraphicsProtocol::ITerm2 => {
            let img = buffer_to_image(buf, upscale);
            let png = match png::encode_png(&img) { Ok(p) => p, Err(_) => return Vec::new() };
            iterm2::encode_iterm2(&png)
        }
        GraphicsProtocol::Kitty => {
            let img = buffer_to_image(buf, upscale);
            let png = match png::encode_png(&img) { Ok(p) => p, Err(_) => return Vec::new() };
            kitty::encode_kitty(&png)
        }
    }
}
```

- [ ] **Step 4: Run + commit**

```bash
cargo test -p tplot-render --lib graphics
git add crates/tplot-render/
git commit -m "Add render_graphics dispatch over GraphicsProtocol"
```

---

## Task 6: CLI — `--graphics` flag

**Files:**
- Modify: `crates/tplot/src/cli.rs`

Add a `--graphics auto|kitty|iterm2|none` value to `CommonStoryArgs` (already shared by every chart subcommand). Default `none`.

- [ ] **Step 1: Write failing test**

```rust
// crates/tplot/src/cli.rs (within existing tests module)
#[test]
fn parses_graphics_flag() {
    let args = Cli::parse_from([
        "tplot", "bar", "sales.csv",
        "-x", "quarter", "-y", "revenue",
        "--graphics", "kitty",
    ]);
    match args.command {
        Command::Bar(b) => assert_eq!(b.common.graphics, "kitty"),
        _ => panic!(),
    }
}

#[test]
fn graphics_flag_defaults_to_none() {
    let args = Cli::parse_from([
        "tplot", "bar", "sales.csv",
        "-x", "quarter", "-y", "revenue",
    ]);
    match args.command {
        Command::Bar(b) => assert_eq!(b.common.graphics, "none"),
        _ => panic!(),
    }
}
```

- [ ] **Step 2: Add the field**

```rust
// crates/tplot/src/cli.rs — modify CommonStoryArgs
#[derive(Args, Debug, Clone)]
pub struct CommonStoryArgs {
    // existing fields...

    /// Render the chart via a terminal graphics protocol when supported.
    /// One of: `auto` (pick the best detected), `kitty`, `iterm2`, `none`.
    #[arg(long, default_value = "none")]
    pub graphics: String,
}
```

- [ ] **Step 3: Run + commit**

```bash
cargo test -p tplot --lib cli
git add crates/tplot/
git commit -m "Add --graphics flag to CommonStoryArgs"
```

---

## Task 7: Pipeline helper — resolve `--graphics auto|kitty|iterm2|none` against caps

**Files:**
- Modify: `crates/tplot/src/pipeline.rs`

Translate the string flag into a `GraphicsProtocol`, with `auto` consulting the detected `Capabilities`:

- `--graphics auto`  → use `caps.graphics_protocol` (which Plan 7 already populated via probing)
- `--graphics kitty` → force Kitty (error if caps say Kitty isn't supported and the user wants strictness — for v1 just trust the user)
- `--graphics iterm2` → force iTerm2
- `--graphics none` → no graphics (text rendering)

For v1, **trust the user when they force a protocol**: even if probing said no, send the escapes anyway. They asked for it.

- [ ] **Step 1: Write failing tests**

```rust
// crates/tplot/src/pipeline.rs (within existing tests module — add if not present)
#[cfg(test)]
mod tests {
    use super::*;
    use tplot_protocol::{Capabilities, ColorDepth, GlyphSet, GraphicsProtocol, Theme};

    fn caps_with(g: GraphicsProtocol) -> Capabilities {
        Capabilities {
            color_depth:       ColorDepth::Truecolor,
            glyph_set:         GlyphSet::Octants,
            graphics_protocol: g,
            theme:             Theme::Dark,
        }
    }

    #[test]
    fn resolve_none_returns_none() {
        assert_eq!(resolve_graphics("none", caps_with(GraphicsProtocol::Kitty)), GraphicsProtocol::None);
    }

    #[test]
    fn resolve_kitty_forces_kitty() {
        assert_eq!(resolve_graphics("kitty", caps_with(GraphicsProtocol::None)), GraphicsProtocol::Kitty);
    }

    #[test]
    fn resolve_iterm2_forces_iterm2() {
        assert_eq!(resolve_graphics("iterm2", caps_with(GraphicsProtocol::None)), GraphicsProtocol::ITerm2);
    }

    #[test]
    fn resolve_auto_uses_caps() {
        assert_eq!(resolve_graphics("auto", caps_with(GraphicsProtocol::Kitty)),  GraphicsProtocol::Kitty);
        assert_eq!(resolve_graphics("auto", caps_with(GraphicsProtocol::ITerm2)), GraphicsProtocol::ITerm2);
        assert_eq!(resolve_graphics("auto", caps_with(GraphicsProtocol::None)),   GraphicsProtocol::None);
    }
}
```

- [ ] **Step 2: Run (expected fail)**

- [ ] **Step 3: Implement**

```rust
// crates/tplot/src/pipeline.rs (append)
use tplot_protocol::GraphicsProtocol;

pub fn resolve_graphics(flag: &str, caps: Capabilities) -> GraphicsProtocol {
    match flag {
        "auto"   => caps.graphics_protocol,
        "kitty"  => GraphicsProtocol::Kitty,
        "iterm2" => GraphicsProtocol::ITerm2,
        "none" | "" => GraphicsProtocol::None,
        // Unknown values default to None (don't crash). The CLI parser accepts
        // any string here; if needed, tighten with clap value-parser later.
        _ => GraphicsProtocol::None,
    }
}
```

- [ ] **Step 4: Run + commit**

```bash
cargo test -p tplot --lib pipeline
git add crates/tplot/
git commit -m "Add pipeline::resolve_graphics for --graphics flag dispatch"
```

---

## Task 8: Wire graphics output into all chart pipelines

**Files:**
- Modify: every `crates/tplot/src/commands/*.rs` (bar, histogram, line, scatter, boxplot, heatmap, stacked_area; sparkline doesn't have a meaningful graphics path)

In each pipeline, after building the `PixelBuffer`, branch:

- If `resolve_graphics(opts.graphics, caps)` returns `GraphicsProtocol::None`, take the existing text path (compose y-axis labels, render half-blocks/braille/etc., output text + takeaway).
- Otherwise, write the graphics bytes (PNG-encoded image escapes) directly, then output the takeaway as a separate text line below.

Add `graphics: String` to each `*Options` struct (mirrors the CLI flag).

- [ ] **Step 1: Update one command first (bar) and write a quick smoke test**

In `crates/tplot/src/commands/bar.rs`:

```rust
// add to RenderOptions
pub struct RenderOptions {
    // existing fields...
    pub graphics: String,
}
```

In `render_bar` (and `render_vertical_bar`), after rasterizing, before composing the text body:

```rust
// existing rasterize step produces `buf`.
let caps = Capabilities::from_vars(|name| std::env::var(name).ok());
let protocol = crate::pipeline::resolve_graphics(&opts.graphics, caps);

if protocol != GraphicsProtocol::None {
    // Graphics path: emit PNG escapes, then the takeaway as a text line.
    let mut out = Vec::new();
    out.extend(tplot_render::graphics::render_graphics(&buf, protocol, /*upscale*/ 6));
    out.push(b'\n');
    if let Some(t) = story.takeaway {
        out.extend(t.as_bytes());
        out.push(b'\n');
    }
    return Ok(String::from_utf8_lossy(&out).to_string());
}

// Existing text path follows.
```

`upscale: 6` is a reasonable default — it turns a typical 80-cell-wide chart (≈80 sub-pixels) into ~480px wide. Tune later if charts look too small/large.

Add the `graphics` field to `RenderOptions { ... }` in main.rs and the test setup.

- [ ] **Step 2: Smoke test bar with `--graphics iterm2`**

```bash
cargo run -p tplot -- bar tests/fixtures/sales.csv -x quarter -y revenue --group region --graphics iterm2 \
  | head -c 200
```

You should see `\x1b]1337;File=inline=1;size=...:<base64...>` bytes. In an actual iTerm2 window, this would render the chart as a real image.

- [ ] **Step 3: Repeat for histogram, line, scatter, boxplot, heatmap, stacked_area**

Each has a similar shape: there's a `let mut buf = PixelBuffer::new(...)` followed by a `let caps = Capabilities::from_vars(...)` followed by a `render_*(&buf, caps)` text rendering call. Insert the graphics-or-text branch in the same place.

For sparkline, skip — the binary output is a single line that doesn't benefit from being a full PNG.

- [ ] **Step 4: Run + smoke test all chart types**

```bash
cargo test --workspace
cargo build --workspace
cargo clippy --workspace --all-targets -- -D warnings

# Smoke each chart with --graphics iterm2 and confirm OSC bytes appear.
for cmd in "bar tests/fixtures/sales.csv -x quarter -y revenue --group region" \
           "hist tests/fixtures/sales.csv -y revenue" \
           "line tests/fixtures/sales.csv -x revenue -y revenue" \
           "scatter tests/fixtures/sales.csv -x revenue -y revenue" \
           "box tests/fixtures/sales.csv -x quarter -y revenue" \
           "area tests/fixtures/sales.csv -x revenue -y revenue --group region"; do
  echo "=== $cmd ==="
  cargo run -p tplot -- $cmd --graphics iterm2 2>&1 | head -c 100
  echo
done
```

- [ ] **Step 5: Commit (per chart type or all together — your choice)**

```bash
git add crates/tplot/
git commit -m "Wire --graphics into all chart pipelines"
```

---

## Task 9: Integration tests for graphics output

**Files:**
- Create: `crates/tplot/tests/e2e_graphics.rs`

Run the binary with `--graphics iterm2` and `--graphics kitty`; confirm the output starts with the expected escape sequence prefix. Don't snapshot the exact bytes (PNG output can vary across `image` versions); test the structure.

- [ ] **Step 1: Write the tests**

```rust
// crates/tplot/tests/e2e_graphics.rs
use std::process::{Command, Stdio};

fn binary_path() -> &'static str { env!("CARGO_BIN_EXE_tplot") }
fn workspace_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(2).unwrap().to_path_buf()
}

fn run(args: &[&str]) -> Vec<u8> {
    Command::new(binary_path())
        .args(args).current_dir(workspace_root())
        .stdout(Stdio::piped()).stderr(Stdio::piped())
        .output().expect("tplot binary failed")
        .stdout
}

#[test]
fn iterm2_graphics_starts_with_correct_osc() {
    let out = run(&[
        "bar", "tests/fixtures/sales.csv",
        "-x", "quarter", "-y", "revenue", "--group", "region",
        "--width", "60",
        "--graphics", "iterm2",
    ]);
    assert!(out.starts_with(b"\x1b]1337;File="),
        "expected iTerm2 OSC prefix; got: {:?}", &out[..out.len().min(40)]);
}

#[test]
fn kitty_graphics_starts_with_correct_apc() {
    let out = run(&[
        "bar", "tests/fixtures/sales.csv",
        "-x", "quarter", "-y", "revenue", "--group", "region",
        "--width", "60",
        "--graphics", "kitty",
    ]);
    assert!(out.starts_with(b"\x1b_Ga=T"),
        "expected Kitty APC prefix; got: {:?}", &out[..out.len().min(40)]);
}

#[test]
fn graphics_none_produces_text_rendering() {
    let out = run(&[
        "bar", "tests/fixtures/sales.csv",
        "-x", "quarter", "-y", "revenue", "--group", "region",
        "--width", "60",
        // no --graphics flag → defaults to "none" → text path.
    ]);
    let s = String::from_utf8_lossy(&out);
    // Text path emits ANSI color escapes, not the iTerm2/Kitty image escapes.
    assert!(!s.starts_with("\x1b]1337"));
    assert!(!s.starts_with("\x1b_G"));
    // EMEA label should appear (story-pass focal).
    assert!(s.contains("EMEA"));
}
```

- [ ] **Step 2: Run + commit**

```bash
cargo test -p tplot --test e2e_graphics
git add crates/tplot/
git commit -m "Add e2e tests for graphics protocol output"
```

---

## Task 10: Update doctor + README + lint pass

**Files:**
- Modify: `crates/tplot/src/commands/doctor.rs` — drop the "(planned in Plan 7b)" recommendation
- Modify: `README.md`

- [ ] **Step 1: Update doctor recommendation**

In `format_report`, replace the "planned in Plan 7b" hint with a current-tense recommendation:

```rust
if matches!(caps.graphics_protocol, GraphicsProtocol::None) {
    let _ = writeln!(out, "  • Text rendering only — no graphics-protocol output is supported.");
} else {
    let _ = writeln!(out, "  • Run with --graphics auto for high-fidelity image rendering");
    let _ = writeln!(out, "    (encodes a PNG and emits {} escapes).", graphics_label(caps.graphics_protocol));
}
```

Re-run / re-accept the doctor snapshots:
```bash
cargo test -p tplot --test e2e_doctor 2>&1 || true
cargo insta accept
```

- [ ] **Step 2: Update README**

Add to "What's in this version" (Plans 1+2+3+4+4.5+5+5.5+6+7+7b). Quickstart snippet:

```bash
# Auto-pick the best graphics protocol your terminal supports
tplot bar metrics.csv -x quarter -y revenue --group region --graphics auto

# Force a specific protocol (useful when probing fails)
tplot bar metrics.csv -x quarter -y revenue --group region --graphics kitty
```

- [ ] **Step 3: Lint pass**

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

- [ ] **Step 4: Commit**

```bash
git add README.md crates/tplot/
git add -A   # if fmt/clippy made changes
git commit -m "Update doctor recommendation and README for graphics output, lint pass"
```

---

## Done — Plan 7b deliverable

```bash
# In iTerm2 or WezTerm-with-iTerm2-mode:
tplot bar tests/fixtures/sales.csv -x quarter -y revenue --group region --graphics auto
# → renders the chart as an actual inline PNG; takeaway prints below
```

```bash
# Force a protocol:
tplot bar ... --graphics kitty
tplot bar ... --graphics iterm2
```

```bash
# Verify which protocol your terminal speaks:
tplot doctor
```

**Still missing for v1**: distribution (Plan 8). **Sixel deferred** to Plan 7c if/when needed.

## Self-review notes

- image+base64 deps + buffer→image (Task 1) ✓
- PNG encoding (Task 2) ✓
- iTerm2 protocol (Task 3) ✓
- Kitty protocol with chunked base64 (Task 4) ✓
- render_graphics dispatch (Task 5) ✓
- --graphics flag (Task 6) ✓
- resolve_graphics helper (Task 7) ✓
- Pipeline plumbing across all charts (Task 8) ✓
- Integration tests (Task 9) ✓
- doctor recommendation + README + lint (Task 10) ✓
