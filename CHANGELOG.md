# Changelog

All notable changes to Trading Help are documented here.

## [Unreleased]

### Fixed
- Gold, silver and oil now state outright that the price is a futures contract,
  not a spot price, and name the symbol it comes from — above the numbers in the
  instrument panel and on hover in the ticker. A futures quote sits above spot by
  the cost of carry (1.6% for gold when measured on 31 July), so next to any
  popular gold quote the figure read as simply wrong
- The TradingView scripts for gold, silver and oil now chart the same contract the
  price is taken from. They pointed at spot feeds, so the chart showed a different
  instrument than the number displayed above it

## [0.4.4] - 2026-07-31

Prices refresh on their own, and anything that is not current now says how old
it is.

### Added
- Prices in the ticker refresh every 60 seconds on their own. The tape states the
  time carried by the quote itself, not the time you happen to be looking at it —
  futures reach the provider with a delay, so that is the only honest "as of"
- Market data (correlations, indicators, Gold/Silver Ratio) refreshes on its own
  every 5 minutes. Until now it was fetched once at startup and then only when you
  pressed the button, so an app left open all day kept showing morning prices
- The correlation builder states how many shared trading sessions a coefficient
  was computed from

### Fixed
- Stored data shown when live data is unavailable is now marked as archival, with
  the date and time it was saved. Previously it was indistinguishable from current
  data — the reason a price from an earlier session could pass for today's. The
  marker appears only after a download has actually failed, so it no longer
  flashes for a moment on every healthy start
- Every time shown on screen now carries a date, not just an hour. A bare "09:12"
  hid the age of anything older than today
- A hung connection to the price provider no longer freezes market data, briefings
  and tactic generation for as long as the system keeps the connection open; these
  requests now have a time limit
- Price candles are ordered by time as they arrive, so a change in the provider's
  ordering cannot quietly reverse the indicators or the price on screen
- Failures to download market data are now written to the log file. A report about
  wrong or stale prices can now be diagnosed from the log instead of guesswork
- In the correlation builder, editing a ticker field after computing no longer
  relabels the result already on screen with the new symbol
- Pressing "Odśwież dane rynkowe" twice in quick succession no longer starts two
  overlapping downloads
- `to_returns` dropped observations instead of marking gaps, silently shifting every
  later return by one session. Removed
- The strongest correlated pair is chosen among measured pairs only, instead of
  ranking unmeasured and non-finite ones last

### Changed
- Correlation is joined on trading dates instead of series length; pairs with
  different session calendars no longer correlate returns from different days
- A correlation that could not be measured is shown as "—" instead of 0.000, and
  is left out of the AI prompt entirely rather than passed as a placeholder the
  model would comment on
- Internal: a round of price updates in which not a single request completed now
  reports a failure instead of an empty success. Nothing looks different today,
  but the app no longer mistakes "nothing came back" for "nothing to update"
- Release builds install dependencies from the lockfile (`npm ci`), so a published
  binary contains the versions that were tested

### Known issue
- `get_market_context` still fails wholesale when no equity pair has a measured
  correlation (`NoStrongestPair`); the Pine correlation script needs to become
  optional

## [0.4.3] - 2026-07-29

### Added
- You can now interrupt a running analysis or tactic generation. The button
  appears only while the app is working, and stopping is treated as a normal
  choice — no error message, the screen simply returns to its previous state
- The app keeps a local log file and Settings has a button that opens the
  folder holding it. Attach it when reporting a problem: it never contains
  your API key, your prompts or the AI's answers, and nothing is sent anywhere
- The accuracy statistics now say how many stored tactics were left out of the
  calculation, instead of quietly showing a smaller number

### Fixed
- A failing AI request no longer burns three of your daily free-tier requests.
  The app now stops retrying once a failure repeats identically, which is a
  sign that trying again cannot help
- Running out of the daily free-tier limit is now stated as such, with the
  time it resets, instead of suggesting you try again in a moment — which was
  simply untrue
- A news headline containing accented or non-Latin characters could crash the
  part of the app that matches headlines to instruments
- Market data that arrives incomplete is now rejected instead of being shown.
  Previously a partial download could surface as a price of 0 and neutral
  indicators, indistinguishable from a genuinely calm market — and a tactic
  generated from it counted toward the accuracy statistics
- A tactic priced at zero can no longer be recorded, and any such entry
  already saved is excluded from the statistics rather than counted as a win
- Saved tactics are now written in a way that cannot leave the file truncated
  if the app is closed mid-write, and two tactics generated in quick
  succession can no longer overwrite each other
- A dropped connection during an AI request is retried instead of failing
  immediately, and a request that cannot finish in reasonable time now ends
  with a clear message rather than waiting for minutes

### Changed
- Instruments are now described in one place in the backend, in preparation
  for the wider instrument list planned for the next release. Nothing changes
  on screen yet, but analyses and tactics can no longer silently fall back to
  the wrong instrument's data

## [0.4.2] - 2026-07-29

### Fixed
- Instrument analysis and trading tactics failed with a "service unavailable"
  error on every attempt. The AI model the app was using had started refusing
  every request on Google's side, so no amount of retrying could help. The app
  now uses a newer model, and both features work again

## [0.4.1] - 2026-07-27

Quality pass over the whole codebase after a full review of v0.4.0.

### Added
- "Skrypty" view collecting the market-wide TradingView indicators (index
  correlation, Gold/Silver Ratio). They no longer take over the Overview,
  which now shows only the script tied to the instrument you analyzed
- Keyboard navigation in instrument search (arrows, Enter, Escape)
- The link to generate a Gemini API key now opens in your browser instead of
  being plain text you had to retype
- Trading tactics now state the reference price their target/stop levels are
  measured against

### Fixed
- App no longer shows a blank white screen if something goes wrong while
  rendering — you get a readable message and a reload button
- Prices, correlation and indicators could be silently wrong: futures
  occasionally return candles with missing values, and a single one corrupted
  every calculation for that instrument. Such candles are now discarded
- "Odśwież dane rynkowe" looked dead: it gave no feedback and, if the refresh
  failed while data was already on screen, the error was swallowed entirely.
  It now shows progress, the last update time, and surfaces failures
- Error messages no longer dump raw technical output. A rejected API key now
  says so and points at Settings, instead of showing Google's JSON alongside a
  misleading "model overloaded, try again"
- Mistyping a ticker in the correlation builder (e.g. "NASDAQ" instead of
  "^IXIC") now explains the expected format instead of reporting a parse error
- Briefings no longer claim "no recent news" when the news source is simply
  unreachable — the two cases are now told apart
- Correlation is now labeled with what it is correlated against ("Korelacja z
  SP500"), and the correlation builder states which ticker its indicators
  describe
- Removing the API key uses an in-app confirmation instead of a system dialog

### Changed
- Market data is cached for 5 minutes, so a single action no longer refetches
  the same symbols several times. Generating a tactic went from roughly five
  requests to two
- The app now checks for updates every 4 hours, not only at startup

## [0.4.0] - 2026-07-26

### Added
- Arbitrary-pair correlation builder ("Korelacje" view): compute correlation,
  volatility, RSI and MACD for any two tickers on demand, not just the fixed
  watchlist pairs
- Live price and daily % change now included in analytical reports, powering
  a live ticker tape across the top of the app

### Changed
- Full visual redesign to a dense, monospace "Terminal" theme (functional-only
  color, zero border-radius) with tabbed views (Overview / Tactics /
  Correlations / Settings) replacing the old single-page stack of cards
- Instrument selection is now a proper search ("instrument in focus") instead
  of a fixed 2x2 grid of all four instruments at once, decoupled from view
  navigation
- Replaced the three fixed daily briefing slots (morning/afternoon/evening,
  each analyzing all 4 instruments) with on-demand, single-instrument
  analysis — one Gemini call per click instead of four, significantly easier
  on the free-tier rate limit (5/min, 20/day)
- AI commentary for a single instrument now only sees that instrument's own
  data (price, RSI, MACD, volatility). Correlation and Gold/Silver Ratio
  remain visible as a passive "market context" panel, computed independently
  and free of the AI rate limit, but no longer influence the generated text

### Fixed
- News grounding was broken: the general "All News" RSS feed is mostly
  geopolitics and had stopped including article descriptions, so briefings
  almost always reported "no relevant news" for every instrument. Switched to
  topical feeds (Stock Market News, Metals Analysis)
- Keyword matching used plain substring search, so "gold" matched inside
  "Goldman Sachs" and could cause the AI to cite unrelated company news as
  evidence about the price of gold. Matching is now word-boundary aware

### Removed
- The animated 3D background (three.js) — dropped in favor of the flatter,
  denser Terminal aesthetic
- The staleness guard that skipped AI regeneration when Yahoo Finance had no
  new data — it was designed around the old shared 4-instrument snapshot and
  has no natural equivalent for single, deliberate per-instrument requests

## [0.3.0] - 2026-07-25

### Added
- Technical indicators: RSI (Wilder's, 14-period) and MACD (12/26/9) computed
  for every instrument and fed directly into the AI commentary
- Real per-instrument news sentiment scoring (-1 to 1), replacing the
  previous placeholder that was always 0
- AI-selected dynamic Pine Script signal variants (uptrend / downtrend /
  consolidation) per instrument — the AI picks only the variant name based on
  RSI/MACD; the code renders one of three hand-written, pre-validated
  templates, so the generated script is always syntactically valid
- On-demand trading tactic generation: bull/bear/neutral scenario with
  target/stop levels and a plain-language rationale for any instrument,
  always shown with a fixed, non-AI-generated legal disclaimer
- Transparent, immutable backtesting of generated tactics: every tactic is
  automatically checked against real market data 24h and 7 days later, and
  the displayed accuracy track record is built exclusively from outcomes
  that have actually been verified
- AI explainability: citations link specific claims in a briefing to either
  an exact numeric input (RSI, MACD, correlation) or a real news headline —
  a citation is only shown when its headline matches the news feed exactly,
  so the AI can never fabricate a source

### Changed
- Yahoo Finance fetch window increased from 30 to 90 days, since MACD(12,26,9)
  needs at least ~34 data points to compute a signal line
- Gold/Silver Ratio "30 days ago" comparison now finds the candle closest to
  that actual date instead of assuming it's the first candle in the fetch
  window (which stopped being true once the window grew to 90 days)
- Internal code quality: added regression tests, `thiserror`-based typed
  errors (`AiEngineError`, `CommandError`), and an `AiProvider` trait laying
  the groundwork for multi-provider AI support
- Refactored `App.tsx` (470 → 235 lines) into `types.ts`, `constants.tsx`,
  `utils/`, `hooks/`, and one component per file under `components/` — no
  behavior change
- Refactored `ai_engine.rs` and `commands.rs` (~840 and ~660 lines) into
  `ai_engine/` and `commands/` module directories, split by responsibility
  with one-directional dependencies and typed errors preserved — no behavior
  change

### Fixed
- AI prompts no longer claim an indicator "crossed" a threshold — the
  numeric data passed to the model is a single snapshot with no history, so
  that language implied information the model didn't actually have
- Trading tactic entry level is now shown as "at current price" instead of a
  misleading, falsely-precise "+0.00%"

### Security
- Bumped `postcss` to 8.5.23, fixing GHSA-r28c-9q8g-f849 (path traversal via
  `sourceMappingURL`, high severity)

## [0.2.3] - 2026-07-24

### Fixed
- Gemini free-tier rate limit (429 RESOURCE_EXHAUSTED): parallel `tokio::join!`
  calls in `get_full_briefing` replaced with sequential calls spaced by
  `GEMINI_CALL_SPACING`, staying within the 5 requests/minute free-tier limit
- `call_gemini` now parses Google's structured error response and returns a
  clear, user-facing Polish message on rate-limit errors instead of raw JSON
- Retry backoff now honors the `retryDelay` suggested in Google's error
  response, falling back to the previous fixed 2s/4s/8s schedule when absent

### Added
- Live briefing progress feedback: backend emits a `briefing-progress` Tauri
  event before each of the 4 AI calls in `get_full_briefing`; the UI shows
  "Analizuję NASDAQ... (1/4)" instead of a static "Analizuję rynki..." message


# [0.2.2] - 2026-07-24

### Fixed
- Corrected TOML syntax error in Cargo.toml introduced during initial 0.2.0
  version bump, which caused the v0.2.0 and v0.2.1 release builds to fail


## [0.2.1] - 2026-07-24

### Fixed
- Corrected TOML syntax error in Cargo.toml introduced during 0.2.0 version bump


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
