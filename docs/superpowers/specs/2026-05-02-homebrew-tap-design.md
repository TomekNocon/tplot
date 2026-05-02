# Plan B: Homebrew Tap — Design

**Status:** approved 2026-05-02
**Scope:** Plan B only (Homebrew formula via personal tap). Plan A (release binaries) is already shipped at v0.1.0. Plans C (cargo publish) and D (PyPI/uv) are deferred.

## Goal

`brew install tomeknocon/tap/tplot` works on macOS (arm64 + x86_64). Each future tplot release auto-pushes a formula update to the tap repo, so users running `brew upgrade` get the new version without manual intervention.

## Non-Goals

- **homebrew-core promotion** (the official tap, `brew install tplot` no prefix). Has hard requirements (~75 GitHub stars, stable post-1.0, active maintainer). Revisit at v1.0.
- **Linuxbrew formula** — cargo-dist's homebrew formula handles macOS only by default. Linux users use the curl-pipe-sh installer from Plan A.
- **Source-built bottles** — we install our prebuilt tarballs. No build step on the user's Mac.
- **Plans C and D** — `cargo publish` and PyPI wrapper.

## Architecture

### How the tap is populated

cargo-dist owns the formula. On every tag push:

```
git tag v0.1.x → push
  → release.yml fires
  → matrix builds 3 archives (Plan A, unchanged)
  → cargo-dist computes archive SHAs
  → cargo-dist generates Formula/tplot.rb pointing at the just-uploaded tarballs
  → cargo-dist commits Formula/tplot.rb to tomeknocon/homebrew-tap using HOMEBREW_TAP_TOKEN
  → users running `brew upgrade tplot` get the new version
```

The formula contains hardcoded SHAs of the version-specific tarballs. cargo-dist regenerates the formula every release with fresh SHAs — never mutates an old one in place.

### How the user installs

```bash
brew tap tomeknocon/tap                  # one-time, registers the tap
brew install tplot                       # installs the binary
# OR the all-in-one form:
brew install tomeknocon/tap/tplot
```

Subsequent updates: `brew upgrade tplot`.

### Authentication for cross-repo push

cargo-dist's release pipeline runs in `TomekNocon/tplot` but needs to write to `TomekNocon/homebrew-tap`. Default `GITHUB_TOKEN` only has access to the workflow's own repo, so we use a fine-grained PAT.

| Component | Value |
|---|---|
| Token name | `tplot-homebrew-tap-write` |
| Owner | `TomekNocon` |
| Repository access | only `TomekNocon/homebrew-tap` |
| Permission | `Contents: Read and write` (only) |
| Expiration | 1 year (rotate via calendar reminder) |
| Storage | Secret `HOMEBREW_TAP_TOKEN` in `TomekNocon/tplot` |

Blast radius if leaked: attacker can push to the tap repo only — they can't read tplot source, modify other repos, or access user data. Recovery: revoke + regenerate (~1 minute).

GitHub Secrets are encrypted at rest, never appear in workflow logs (cargo-dist's generated YAML doesn't echo the token), and are withheld from fork-PR workflows by default — safe in a public repo.

### Versioning

The first homebrew-shipping release will be **`v0.1.1`**. We do NOT retroactively add a formula to v0.1.0 — that would mean re-running a workflow against an existing tag and confuses the release timeline. v0.1.0 ships without homebrew; v0.1.1 ships with it.

`v0.1.1` is a patch bump because Homebrew distribution is purely additive. No code changes. CHANGELOG entry: "Added Homebrew formula via cargo-dist".

## Components

### 1. dist-workspace.toml updates

Two changes:

```toml
[dist]
installers = ["shell", "homebrew"]   # was: ["shell"]
tap = "TomekNocon/homebrew-tap"
formula = "tplot"
publish-jobs = ["homebrew"]
```

`tap` and `formula` are new keys. `publish-jobs` tells cargo-dist to enable the homebrew-publish step in the workflow.

### 2. Regenerate release.yml

Run `dist generate`. cargo-dist adds a new job that:
- Downloads the build artifacts from the matrix legs
- Computes formula contents
- Authenticates to `TomekNocon/homebrew-tap` via `HOMEBREW_TAP_TOKEN`
- Commits and pushes `Formula/tplot.rb`

We preserve our manual edits:
- `workflow_dispatch:` trigger (added in Plan A)
- `allow-dirty = ["ci"]` in `dist-workspace.toml`

### 3. Bump version + CHANGELOG

`Cargo.toml` workspace version `0.1.0 → 0.1.1`.

`CHANGELOG.md` adds:

```markdown
## [0.1.1] - 2026-05-02

### Added

- Homebrew formula auto-published to `tomeknocon/homebrew-tap` on each
  release. Install via `brew install tomeknocon/tap/tplot`.
```

### 4. README updates

Move the Homebrew section above the curl one-liner — it's the install path most macOS users expect by default. Add a "Linuxbrew" note explaining the formula is macOS-only and Linux users should use curl.

### 5. Tag and ship

`git tag v0.1.1 && git push origin v0.1.1`. Watch for:
- Release workflow goes green (now with a new homebrew-publish job at the end)
- `Formula/tplot.rb` appears in `tomeknocon/homebrew-tap`
- `brew install tomeknocon/tap/tplot` works on a clean shell

## Data Flow (Release with Homebrew)

```
git tag v0.1.1 → git push origin v0.1.1
  → release.yml fires
  → matrix: 3 build legs (mac arm64, mac x86_64, linux x86_64)
  → host-stage: collect artifacts, compute checksums
  → publish-stage: upload assets to github releases (Plan A path)
  → homebrew-stage: NEW
    → generate Formula/tplot.rb with v0.1.1 URLs + SHAs
    → git commit to TomekNocon/homebrew-tap using HOMEBREW_TAP_TOKEN
    → push to main of homebrew-tap
  → users now see v0.1.1 in `brew search` after a `brew update`
```

## Failure Modes

| Symptom | First check |
|---|---|
| Homebrew job fails with "permission denied" pushing to tap | PAT scope wrong — confirm `Contents: write` on `TomekNocon/homebrew-tap` |
| Secret not found error | Secret name mismatch — `HOMEBREW_TAP_TOKEN` exactly (case-sensitive) in `tplot` repo settings |
| `brew install` says "formula not found" | Tap not synced — user must run `brew update` first; confirm `Formula/tplot.rb` exists in tap repo |
| Formula 404s on the tarball download | Race condition: tap pushed before release assets uploaded. Re-run the homebrew-publish job from the Actions UI. |
| Old formula version in tap | `brew update` is cached on user side; they need to refresh tap metadata |
| PAT expires (1 year out) | Workflow fails; rotate per the runbook (regenerate PAT, update secret) |
| User sees `brew install` work but binary is wrong arch | Formula's per-arch logic broken. Open the formula on github.com/TomekNocon/homebrew-tap and inspect — should have separate `if Hardware::CPU.arm?` branches |

## Testing

- **Local pre-flight:** `dist plan` (read-only) prints what the workflow will do without actually running it. Verify it includes a homebrew-publish step.
- **First release smoke test:** After tagging v0.1.1, watch the release workflow. The homebrew-publish job is the LAST step — failures there don't break Plan A's binary releases (those have already uploaded).
- **End-user smoke test:** On a clean shell:
  ```bash
  brew tap tomeknocon/tap
  brew install tplot          # not the long form, to verify the short alias works
  tplot --version             # expect: tplot 0.1.1
  echo "v\n10\n50\n100" | tplot summary - -y v
  ```

## Open Questions

None. All design decisions resolved.
