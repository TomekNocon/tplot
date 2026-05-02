# Plan B: Homebrew Tap — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `homebrew` to cargo-dist's installers, ship a v0.1.1 release that auto-publishes a `Formula/tplot.rb` to `TomekNocon/homebrew-tap`, and verify `brew install tomeknocon/tap/tplot` works for end-users.

**Architecture:** cargo-dist's homebrew installer regenerates the formula on every tag push, computing per-archive SHAs and committing the result to the tap repo via a fine-grained PAT. No source-built bottles. v0.1.0's release page stays untouched; v0.1.1 is the first homebrew-shipping release.

**Tech Stack:** cargo-dist (already installed), GitHub Actions (already configured), Homebrew (no local install needed for the build step), one fine-grained PAT (already generated).

**Inherited context (Plan A):**
- v0.1.0 cut and live; install one-liner works.
- `dist-workspace.toml` exists with 3 targets and `installers = ["shell"]`.
- `.github/workflows/release.yml` exists with hand-edited `workflow_dispatch:` and `allow-dirty = ["ci"]`.
- CI workflow green; rust-toolchain.toml pins channel `1.91`.
- Branches `main` and `feature/foundation` both at `db742d3` ahead of v0.1.0 tag.

**Prerequisites already done by the user (verified before this plan started):**
- `TomekNocon/homebrew-tap` exists, public, with a `README.md`.
- Fine-grained PAT created with `Contents: write` on the tap repo only.
- Secret `HOMEBREW_TAP_TOKEN` saved in `TomekNocon/tplot` repo settings.

---

## File Structure

```
TerminalPlot/
├── Cargo.toml                                EXTEND: bump version 0.1.0 → 0.1.1
├── dist-workspace.toml                       EXTEND: add homebrew installer + tap config
├── .github/workflows/release.yml             REGENERATE: dist generate adds homebrew-publish job
├── CHANGELOG.md                              EXTEND: add ## [0.1.1] entry
└── README.md                                 EDIT: prepend Homebrew section to Install
```

No source code changes. No tests added or removed.

---

## Task 1: Add homebrew config to dist-workspace.toml

**Files:**
- Modify: `dist-workspace.toml`

cargo-dist needs four config values to publish to a tap: the installer name, the tap repo, the formula name, and the publish-job enablement.

- [ ] **Step 1: Inspect current config**

```bash
cat dist-workspace.toml
```

You'll see a `[dist]` table with `installers = ["shell"]` and `targets = [...]`. Note the current line numbers for surgical edits.

- [ ] **Step 2: Edit the [dist] table**

Make these changes to `dist-workspace.toml`'s `[dist]` table:

1. Change `installers = ["shell"]` to `installers = ["shell", "homebrew"]`
2. Add three new lines under `installers`:

```toml
tap = "TomekNocon/homebrew-tap"
formula = "tplot"
publish-jobs = ["homebrew"]
```

The final `[dist]` block should look approximately like (preserving every other key):

```toml
[dist]
cargo-dist-version = "0.31.0"            # whatever was there before, don't touch
ci = "github"                            # whatever was there before, don't touch
installers = ["shell", "homebrew"]
tap = "TomekNocon/homebrew-tap"
formula = "tplot"
publish-jobs = ["homebrew"]
targets = ["aarch64-apple-darwin", "x86_64-apple-darwin", "x86_64-unknown-linux-gnu"]
install-path = "CARGO_HOME"              # whatever was there before
hosting = ["github"]                     # whatever was there before
allow-dirty = ["ci"]                     # whatever was there before — KEEP THIS
```

(Existing `cargo-dist-version`, `ci`, `install-path`, `hosting`, `allow-dirty` lines stay. We're only adding three lines and editing one.)

- [ ] **Step 3: Verify the edit**

```bash
grep -E "(installers|tap|formula|publish-jobs|allow-dirty)" dist-workspace.toml
```

Expected output (order may differ):
```
installers = ["shell", "homebrew"]
tap = "TomekNocon/homebrew-tap"
formula = "tplot"
publish-jobs = ["homebrew"]
allow-dirty = ["ci"]
```

- [ ] **Step 4: No commit yet.** We commit after Task 2 regenerates `release.yml`, in one combined commit. Cargo-dist's invariant is that `dist-workspace.toml` and `.github/workflows/release.yml` should change together.

---

## Task 2: Regenerate release.yml

**Files:**
- Modify: `.github/workflows/release.yml`

cargo-dist's `dist generate` command produces a workflow YAML matching the current `dist-workspace.toml`. This will add a homebrew-publish job downstream of the build matrix.

- [ ] **Step 1: Snapshot the current release.yml so we can diff later**

```bash
cp .github/workflows/release.yml /tmp/release.yml.before
```

- [ ] **Step 2: Run dist generate**

```bash
dist generate                # binary is `dist`, not `cargo dist`, in v0.31
```

Expected: prints something like `Wrote .github/workflows/release.yml`. May warn about the manual `workflow_dispatch:` edit; the `allow-dirty = ["ci"]` config tells cargo-dist to accept it. If it errors with "out-of-date contents", inspect the error message — the contract may have shifted; re-add `ci` to `allow-dirty` if it got dropped.

- [ ] **Step 3: Verify workflow_dispatch is preserved**

```bash
grep -A1 "workflow_dispatch" .github/workflows/release.yml
```

Expected: `workflow_dispatch:` line appears with either `{}` or an inputs block. If the regeneration stripped it, manually add it back under `on:` (it lives alongside `pull_request:` and `push:`).

- [ ] **Step 4: Verify the homebrew-publish job exists**

```bash
grep -n "homebrew" .github/workflows/release.yml | head -5
```

Expected: at least one match referencing a homebrew step or job. If zero matches, `dist generate` didn't pick up the config — re-check Task 1's edits.

- [ ] **Step 5: Verify the secret reference uses the right name**

```bash
grep "HOMEBREW_TAP_TOKEN" .github/workflows/release.yml
```

Expected: at least one line referencing `${{ secrets.HOMEBREW_TAP_TOKEN }}`. If it references a different secret name (e.g. `HOMEBREW_TAP_DEPLOY_TOKEN` — older cargo-dist versions used this), edit `.github/workflows/release.yml` to replace it with `HOMEBREW_TAP_TOKEN` so it matches the secret you've already saved.

- [ ] **Step 6: Diff for visibility**

```bash
diff /tmp/release.yml.before .github/workflows/release.yml | head -50
```

Skim the diff: should be additive (new homebrew job + maybe a new secret reference). No removals of existing matrix legs or steps.

- [ ] **Step 7: Stage and commit Tasks 1+2 together**

```bash
git add dist-workspace.toml .github/workflows/release.yml
git commit -m "Add Homebrew installer + cargo-dist publish-job for tomeknocon/homebrew-tap"
```

---

## Task 3: Bump workspace version to 0.1.1

**Files:**
- Modify: `Cargo.toml`

The release workflow only fires on tag push, and the tag must match the workspace version (cargo-dist hard-fails otherwise).

- [ ] **Step 1: Edit `[workspace.package]`**

In `Cargo.toml`, change:

```toml
version     = "0.1.0"
```

to:

```toml
version     = "0.1.1"
```

(Indentation may vary; just the version string changes.)

- [ ] **Step 2: Verify the bump propagates**

```bash
cargo metadata --no-deps --format-version 1 \
  | python3 -c "import sys,json; p=[x for x in json.load(sys.stdin)['packages'] if x['name']=='tplot'][0]; print('version:', p['version'])"
```

Expected: `version: 0.1.1`.

- [ ] **Step 3: Build sanity check**

```bash
cargo check --workspace
```

Expected: clean. Cargo will rebuild deps because the version changed.

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "Bump workspace version to 0.1.1"
```

(`Cargo.lock` updates because the workspace package version is recorded there.)

---

## Task 4: Add CHANGELOG entry for 0.1.1

**Files:**
- Modify: `CHANGELOG.md`

cargo-dist auto-pulls the matching `## [0.1.1]` section into release notes.

- [ ] **Step 1: Insert the new entry**

Find the line `## [Unreleased]` near the top of `CHANGELOG.md`. Below it (and above `## [0.1.0]`), add:

```markdown
## [0.1.1] - 2026-05-02

### Added

- Homebrew formula auto-published to `tomeknocon/homebrew-tap` on each
  release. Install via `brew install tomeknocon/tap/tplot` (or the
  shorter `brew install tplot` after a one-time `brew tap tomeknocon/tap`).
```

Also add the matching version-comparison footnote at the bottom of the file. Find the existing `[0.1.0]: https://github.com/TomekNocon/tplot/releases/tag/v0.1.0` line. Above the unreleased footnote, add:

```markdown
[0.1.1]: https://github.com/TomekNocon/tplot/releases/tag/v0.1.1
```

And update the `[Unreleased]` footnote to compare against `v0.1.1` instead of `v0.1.0`:

```markdown
[Unreleased]: https://github.com/TomekNocon/tplot/compare/v0.1.1...HEAD
```

- [ ] **Step 2: Verify the structure**

```bash
grep "^##" CHANGELOG.md
```

Expected (in order):
```
## [Unreleased]
## [0.1.1] - 2026-05-02
## [0.1.0] - 2026-05-01
```

- [ ] **Step 3: Commit**

```bash
git add CHANGELOG.md
git commit -m "Add CHANGELOG entry for 0.1.1 (Homebrew distribution)"
```

---

## Task 5: Update README install section

**Files:**
- Modify: `README.md`

Move Homebrew to the top of the install section since it's most macOS users' first instinct.

- [ ] **Step 1: Find the existing Install section**

```bash
grep -n "^##" README.md | head
```

Note the line number of `## Install`.

- [ ] **Step 2: Replace the `## Install` block**

Replace the entire block from `## Install` down to (but not including) the next `## ` heading (likely `## Quickstart`) with:

```markdown
## Install

### Homebrew (macOS)

```bash
brew install tomeknocon/tap/tplot
```

Or as a tap-and-install:

```bash
brew tap tomeknocon/tap
brew install tplot
```

Updates: `brew upgrade tplot`. The formula is auto-published from each
release; you'll see new versions after the next `brew update`.

### One-liner (curl, macOS + Linux)

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/TomekNocon/tplot/releases/latest/download/tplot-installer.sh | sh
```

This grabs the right prebuilt binary for your platform, drops it on
your `PATH`, and is the fastest install on Linux. On macOS, prefer
Homebrew above unless you have a reason not to.

### Manual download

Pick the archive matching your platform from the [latest release](https://github.com/TomekNocon/tplot/releases/latest):

| Platform | Archive |
|---|---|
| macOS, Apple silicon | `tplot-aarch64-apple-darwin.tar.xz` |
| macOS, Intel         | `tplot-x86_64-apple-darwin.tar.xz` |
| Linux, x86_64 (gnu)  | `tplot-x86_64-unknown-linux-gnu.tar.xz` |

```bash
curl -L https://github.com/TomekNocon/tplot/releases/latest/download/tplot-aarch64-apple-darwin.tar.xz \
  | tar xJ
sudo mv tplot /usr/local/bin/
```

Verify checksums against the per-archive `*.sha256` files published with each release.

### From source

```bash
cargo install --path crates/tplot
```
```

- [ ] **Step 3: Verify the structure**

```bash
grep -n "^### " README.md | head
```

Expected: at least four lines for the new sub-headings — `### Homebrew (macOS)`, `### One-liner (curl, macOS + Linux)`, `### Manual download`, `### From source`.

- [ ] **Step 4: Commit**

```bash
git add README.md
git commit -m "Move Homebrew to top of README install section, document brew install"
```

---

## Task 6: Pre-flight `dist plan`

cargo-dist's `dist plan` is read-only — it shows what the workflow WILL do without actually doing it. Useful for verifying our config changes resulted in the workflow we expect.

- [ ] **Step 1: Run dist plan**

```bash
dist plan
```

Expected output: a summary of the planned release with three matrix legs AND a homebrew-publish step. If `dist plan` doesn't mention homebrew, our config didn't take effect — check Task 1.

- [ ] **Step 2: No commit needed.**

---

## Task 7: Push, tag, and ship

**This task is operator-driven.** Subagents stop after Task 6.

- [ ] **Step 1: Push the new commits to origin/main**

```bash
git push origin main
```

This triggers the CI workflow (test/clippy/fmt) on push — verify it goes green before tagging. CI failure here means we broke something with the version bump or config edits and shouldn't tag.

- [ ] **Step 2: Tag and push v0.1.1**

```bash
git tag v0.1.1
git push origin v0.1.1
```

The release workflow fires on tag push.

- [ ] **Step 3: Watch the release workflow**

Open `https://github.com/TomekNocon/tplot/actions`. You should see a new "Release" run. It runs:
- 3 matrix build legs (mac arm64, mac x86_64, linux x86_64) — same as v0.1.0
- A "host" stage that uploads release assets to GitHub
- A NEW "homebrew" publish stage that pushes Formula/tplot.rb to the tap repo

The homebrew stage is LAST. If it fails, the v0.1.1 release page on GitHub is still complete (binaries are already uploaded). You'd just retry the homebrew job.

- [ ] **Step 4: Verify the formula appeared in the tap repo**

Open `https://github.com/TomekNocon/homebrew-tap` in a browser. You should see a new commit pushing `Formula/tplot.rb`. The commit author will be a bot identity (e.g. `github-actions[bot]`) attributed by cargo-dist.

```bash
gh api repos/TomekNocon/homebrew-tap/contents/Formula/tplot.rb -q .name
```

Expected: `tplot.rb`. (404 means cargo-dist didn't push — check the homebrew-publish job log.)

- [ ] **Step 5: End-user smoke test**

On a fresh shell (or any shell with brew installed):

```bash
brew tap tomeknocon/tap
brew install tplot
tplot --version
printf 'value\n10\n50\n100\n200\n' | tplot summary - -y value
```

Expected:
- `tplot --version` prints `tplot 0.1.1`
- The summary chart prints exactly one line, focal-orange `200` highlight

If anything fails, paste the brew error and we patch.

---

## Failure-Mode Reference

| Symptom | First check |
|---|---|
| `dist generate` errors with "out-of-date" | re-add `"ci"` to `allow-dirty` in dist-workspace.toml; rerun |
| Workflow fails: "secret HOMEBREW_TAP_TOKEN not found" | verify secret name exactly matches what release.yml expects (case-sensitive) |
| Homebrew publish job fails: "permission denied" | PAT scope wrong — confirm `Contents: write` on `TomekNocon/homebrew-tap` (not other repos, not just metadata) |
| Formula pushed but `brew install` says "not found" | end-user needs `brew update` first to refresh tap metadata after the formula is added |
| Wrong arch installed | inspect the formula in the tap repo — should have `if Hardware::CPU.arm?` branches with separate URLs per arch |
| Old workflow ran on v0.1.1 push (no homebrew job) | wrong workflow ref — verify main branch was pushed BEFORE the tag (workflow file is read from default branch) |
| `Cargo.lock` shows uncommitted changes after version bump | `cargo build` updated the lockfile; just stage it as part of Task 3 |

## Self-Review Notes

- ✓ All 5 components from the spec covered (config, regenerate, version, changelog, README, ship).
- ✓ Each task self-contained with exact commands and expected outputs.
- ✓ No placeholders.
- ✓ Type/name consistency: `HOMEBREW_TAP_TOKEN`, `tomeknocon/homebrew-tap`, `v0.1.1` used uniformly.
- ✓ Operator-only steps (Task 7) clearly flagged so subagents don't accidentally push tags.
- ✓ Failure modes inherited from cargo-dist quirks (out-of-date contents, secret naming) called out — these have bitten us before during Plan A.
