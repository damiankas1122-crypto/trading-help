# Changelog

All notable changes to Trading Help are documented here.

## [0.2.0] - 2026-07-24

### Added
- Secure, in-app Gemini API key onboarding — first-launch screen prompts for
  the key instead of requiring a manual environment variable
- API key stored exclusively in the OS-native credential store (Windows
  Credential Manager) via the `keyring` crate — never exposed to the
  frontend, never written in plain text
- "Zmień klucz API" button in the app header to view/replace the stored key
  at any time without leaving the app

### Changed
- `ai_engine.rs` no longer reads `GEMINI_API_KEY` from the environment;
  the key is now read from the OS credential store on every Gemini call
- README updated to reflect the new onboarding-based key setup, replacing
  the old environment variable instructions

### Security
- Gemini API key never touches the JavaScript/frontend layer at any point


## [0.1.5] - 2026-07-23

### Added
- Legal disclaimer footer ("not investment advice") in the app UI, dismissible
  per session via an "I understand" button.

### Security
- Restricted the Content Security Policy in `tauri.conf.json` (previously `null`).
- Removed the unused `shell:allow-open` capability.
- Pinned all GitHub Actions in the release pipeline to commit SHA instead of
  floating version tags.
- Added automated dependency auditing to CI (`npm audit`, `cargo audit`,
  `gitleaks`), running on every push/PR and weekly on a schedule.
- Added Dependabot configuration for npm, Cargo, and GitHub Actions updates.
- Added `SECURITY.md` with a vulnerability reporting policy.
- Enabled branch and tag protection rules on GitHub to prevent force-pushes
  and deletion of `main` and release tags.


## [0.1.4] - 2026-07-22

### Verified
- Confirmed the auto-update pipeline works end-to-end after the capabilities
  fix in 0.1.3: an installed 0.1.3 build detects, downloads, verifies, and
  installs this release automatically via the in-app update banner.

## [0.1.3] - 2026-07-22

### Fixed
- Auto-update was silently failing in production builds because the
  `updater` and `process` plugin permissions were missing from
  `src-tauri/capabilities/default.json`. Tauri 2's capability system blocks
  unlisted plugin commands at the frontend boundary, so `check()` and
  `downloadAndInstall()` were failing silently (caught and only logged to
  the console) instead of showing the update banner. Added
  `updater:default` and `process:default` to the default capability.

## [0.1.2] - 2026-07-22

### Added
- Documented the auto-update mechanism in the README and this changelog, as
  part of verifying the end-to-end update flow (install v0.1.1 → publish
  v0.1.2 → confirm the running app detects and installs it).

## [0.1.1] - 2026-07-22

### Added
- Signed auto-update support via `tauri-plugin-updater` and
  `tauri-plugin-process`. The app now checks GitHub Releases on startup and
  can download, cryptographically verify, and install new versions without a
  manual reinstall.
- In-app update banner (`useAppUpdater` hook in `App.tsx`) showing update
  availability, download progress, and install status.
- GitHub Actions release workflow (`.github/workflows/release.yml`) that
  builds, signs, and publishes a new release whenever a `v*` tag is pushed.

### Fixed
- `ai_engine.rs`: Gemini API calls now retry automatically (up to 3 attempts,
  exponential backoff) on `503` (model overloaded) and `429` (rate limited)
  responses, instead of surfacing a raw error to the user on the first
  transient failure.

## [0.1.0] - Initial release

Initial public release: cross-market correlation analysis, Gold/Silver Ratio
tracking, AI-generated briefings via Gemini, and TradingView Pine Script
generation.
