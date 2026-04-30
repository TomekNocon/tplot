# Plan A: Cross-Platform Release Binaries — Design

**Status:** approved 2026-04-30
**Scope:** Plan A only (distribution binaries on GitHub Releases). Plans B/C/D (Homebrew, crates.io, PyPI) are deferred.

## Goal

A `git tag v0.1.0 && git push origin v0.1.0` produces a GitHub Release with prebuilt `tplot` binaries for three platforms. Users install in one command via `curl … | sh`.

## Non-Goals

Out of scope for this plan, will be revisited later:

- Homebrew tap (Plan B)
- `cargo publish` to crates.io (Plan C)
- Python wrapper / `uv add tplot` (Plan D)
- Sigstore / cosign / GPG asset signing
- Linux ARM, Linux musl, Windows, FreeBSD targets
- Nightly or pre-release channels
- Auto-bumping version via release-please / cargo-release

## Targets

Three native architectures, lean by design:

| target triple | runner | rationale |
|---|---|---|
| `aarch64-apple-darwin` | `macos-14` (Apple silicon) | dominant Mac architecture in 2026 |
| `x86_64-apple-darwin` | `macos-13` (Intel) | Intel Macs still common in enterprise fleets |
| `x86_64-unknown-linux-gnu` | `ubuntu-latest` | covers ~95% of Linux terminal users |

Adding Linux ARM, musl, or Windows is a one-line config change in `dist-workspace.toml` once needed; we deliberately defer.

## Architecture

### Workflow generator: cargo-dist

`cargo-dist` (axodotdev/cargo-dist) generates and owns the release pipeline. We accept its defaults:

- Archive format: `.tar.xz` on unix
- Checksums: SHA256SUMS file alongside archives
- Auto-generated `tplot-installer.sh` (POSIX shell) that detects the user's platform and downloads the correct archive
- Release notes: pulled from `CHANGELOG.md` `## [X.Y.Z]` section matching the tag

Why cargo-dist over hand-rolled YAML: at the cost of a generated workflow, we get the slick `curl --proto '=https' --tlsv1.2 -LsSf … | sh` install line for free, and adding Homebrew (Plan B) becomes a single config flag.

### Release trigger

Two ways to fire the release pipeline:

1. **`git tag vX.Y.Z && git push origin vX.Y.Z`** — the canonical path
2. **`workflow_dispatch`** — manual trigger from the GitHub Actions UI for testing the pipeline without committing a tag

### CI workflow (separate)

A hand-written `.github/workflows/ci.yml` runs on every push to `main` and every PR. Three jobs in parallel:

- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo fmt --all --check`

This is independent of the release workflow and exists to keep `main` healthy regardless of when releases happen.

### Versioning

First release: **`v0.1.0`** — matches the current `workspace.package.version`. We are pre-1.0 by intent: the CLI surface (flags, output format) may change, and ripgrep/bat/fd all spent years in 0.x without issue.

Future bumps follow conventional semver:
- Breaking flag rename / output format change → minor (0.1 → 0.2)
- New flags / new chart types → patch (0.1.0 → 0.1.1)
- "Stable enough" declaration → 1.0.0 (deferred)

### Changelog

`CHANGELOG.md` in [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) format. cargo-dist plucks the matching version section into release notes automatically. Initial `## [0.1.0]` entry summarizes the 16 chart types and the inline-Claude-Code design philosophy.

## Components

### 1. Crate metadata polish

The workspace currently has minimal metadata (`version`, `edition`, `license`, `repository`). For cargo-dist's installer + GitHub Releases UX, we add at workspace level:

```toml
[workspace.package]
description = "Storytelling-first terminal data visualization."
homepage    = "https://github.com/TomekNocon/tplot"
readme      = "README.md"
keywords    = ["chart", "plot", "cli", "terminal", "data"]
categories  = ["command-line-utilities", "visualization"]
authors     = ["Tomek Nocon <tomek.lm10@gmail.com>"]
```

`crates/tplot/Cargo.toml` inherits via `description.workspace = true`, etc.

License files: `LICENSE-MIT` and `LICENSE-APACHE` at repo root (standard MIT and Apache 2.0 templates from rust-lang/api-guidelines).

### 2. CHANGELOG.md

Initial structure:

```markdown
# Changelog

All notable changes to tplot are documented here.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) · semver.

## [Unreleased]

## [0.1.0] — 2026-04-30
### Added
- 16 chart types: bar, hist, line, scatter, sparkline, heatmap, box, area,
  candlestick, treemap, violin, ridgeline, sankey, table, summary
- Storytelling layer: focal/context palette, trust-score gate, embedded takeaway
- Hybrid renderers: half-blocks, vertical-blocks, Braille
- Image protocols: Kitty, iTerm2 (auto-detected via OSC probing)
- Single-line `tplot summary` for inline Claude Code rendering
- JSON ChartSpec dispatch via `tplot json`
- 348 tests across the workspace
```

### 3. cargo-dist setup

```bash
cargo install cargo-dist
cargo dist init
```

The wizard asks about hosting (GitHub Releases), targets (we pick the 3), installers (shell), CI (GitHub Actions). Output: `dist-workspace.toml` at repo root, `.github/workflows/release.yml` generated.

We commit the generated files unmodified except for any obvious ergonomic tweaks (e.g. ensure `workflow_dispatch` is included, which is on by default).

### 4. CI workflow

Hand-written `.github/workflows/ci.yml`, ~30 lines:

```yaml
name: CI
on:
  push:
    branches: [main]
  pull_request:

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo test --workspace --all-targets
      - run: cargo clippy --workspace --all-targets -- -D warnings
      - run: cargo fmt --all --check
```

We run the three checks in a single job (sequentially) rather than parallel jobs, because the test suite is small (<10s) and avoids paying the toolchain-install cost three times.

### 5. README install section

cargo-dist's auto-generated curl-pipe-sh as the headline:

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/TomekNocon/tplot/releases/latest/download/tplot-installer.sh | sh
```

Plus a manual fallback per platform for users who don't want to pipe a script:

```bash
# macOS arm64
curl -L https://github.com/TomekNocon/tplot/releases/latest/download/tplot-aarch64-apple-darwin.tar.xz \
  | tar xJ
sudo mv tplot /usr/local/bin/

# (and similar for x86_64 Mac and Linux x86_64)
```

### 6. First-release dry run + tag push

The plan ends with cutting `v0.1.0`:

```bash
git tag v0.1.0
git push origin v0.1.0
```

Watch the Actions tab; the release should appear at `github.com/TomekNocon/tplot/releases/v0.1.0` with 4 archives (3 `.tar.xz` + `tplot-installer.sh`) and a `SHA256SUMS` file.

## Data Flow (Release)

```
git tag v0.1.0 → git push origin v0.1.0
  → release.yml fires
  → matrix job: 3 native runners build in parallel
  → cargo build --release --target <triple> -p tplot
  → cargo-dist packages: tplot-{aarch64-apple-darwin,x86_64-apple-darwin,x86_64-unknown-linux-gnu}.tar.xz
  → cargo-dist generates: tplot-installer.sh
  → cargo-dist computes: SHA256SUMS
  → all 5 assets uploaded to GitHub Release v0.1.0
  → release notes ← CHANGELOG.md ## [0.1.0]
```

## Failure Modes

| failure | mitigation |
|---|---|
| Repo doesn't exist on GitHub at tag-push time | Already created at `git@github.com:TomekNocon/tplot.git` (Task 0); both branches pushed |
| One target build fails | cargo-dist aborts the release atomically — no half-published releases |
| Tag pushed without a matching CHANGELOG entry | cargo-dist emits empty release notes (acceptable degraded mode); CI doesn't block on this |
| Pre-tag pipeline change broke something | `workflow_dispatch` lets you test the whole pipeline without cutting a real tag |
| Network flake during a single target's build | re-run that matrix leg from the Actions UI |
| Wrong version in Cargo.toml vs git tag | cargo-dist hard-fails with a clear message; fix the workspace version and re-tag |

## Testing

- **Local:** `cargo dist build` packages the current workspace into archives in `target/distrib/` without requiring GitHub Actions; sanity-checks the build matrix offline.
- **Pipeline first run:** Trigger via `workflow_dispatch` against the existing branch before tagging — confirms runners + permissions work without burning a real version number.
- **Release smoke test:** After `v0.1.0` lands, run the install one-liner on a fresh shell and verify `tplot --version` prints `tplot 0.1.0`.

## Open Questions

None. All design decisions resolved during brainstorming.
