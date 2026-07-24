# Trading Help — AI Market Co-Pilot

A free, open-source desktop application that analyzes NASDAQ, S&P 500, Gold,
and Silver in real time, correlates them with live financial news, and turns
the results into plain-language market commentary plus ready-to-use
TradingView Pine Script indicators.

Built with **Rust (Tauri 2)** on the backend and **React + TypeScript** on
the frontend. Actively developed and used for daily market briefings.

## Download

Grab the latest Windows installer (no build required) from the
[Releases page](https://github.com/damiankas1122-crypto/trading-help/releases/latest).
The app checks for updates automatically on launch — no manual reinstalls
needed after the first install.

## Features

- **Cross-market correlation** — computes lagged correlation between NASDAQ
  and S&P 500 returns, so you can see which index tends to lead the other.
- **Precious metals analysis** — tracks the Gold/Silver Ratio (GSR) over a
  30-day window, including its correlation and rate of change.
- **Live news integration** — pulls a financial RSS feed and filters it by
  instrument, so market commentary is grounded in what's actually being
  reported.
- **AI-generated briefings** — combines the numeric analysis with relevant
  news into a readable, per-instrument summary (via the Google Gemini API).
- **Session-to-session comparison** — every run is saved locally, so each new
  briefing highlights what changed since the last one (morning vs. afternoon
  vs. evening).
- **Duplicate-briefing guard** — if market data hasn't actually changed since
  the last snapshot (e.g. re-running within the same session, or over a
  weekend), the app skips the AI call entirely and shows a clear notice
  instead of burning API quota on a near-identical report.
- **One-click TradingView scripts** — generates and explains Pine Script v6
  indicators for index correlation and the Gold/Silver Ratio, ready to paste
  into TradingView.
- **Secure, in-app API key setup** — no config files or environment
  variables to edit. Paste your Gemini API key once in a simple onboarding
  screen; it's stored exclusively in your operating system's native
  credential store (Windows Credential Manager) and is never exposed to the
  app's frontend or written to disk in plain text.
- **Signed auto-updates** — the installed app checks GitHub Releases on
  startup and can download, cryptographically verify, and install new
  versions in place.

## Tech stack

| Layer       | Technology                                 |
| ----------- | ------------------------------------------- |
| Backend     | Rust, Tauri 2                               |
| Frontend    | React, TypeScript, Vite, Tailwind CSS        |
| Market data | Yahoo Finance API                           |
| AI          | Google Gemini API (`gemini-3.5-flash`)      |
| News        | RSS feed parsing                            |
| Credentials | OS-native credential store (via `keyring`)  |
| Storage     | Local JSON snapshot (no external database)  |

## Getting started

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (stable toolchain)
- [Node.js](https://nodejs.org/) 18+
- A [Gemini API key](https://aistudio.google.com/apikey) — the free tier is
  enough for personal use

### Setup

```bash
# Clone the repository
git clone https://github.com/damiankas1122-crypto/trading-help.git
cd trading-help

# Install frontend dependencies
npm install
```

### Gemini API key

No manual configuration required. On first launch, the app shows an
onboarding screen prompting you to paste your Gemini API key. From that
point on:

- The key is saved securely in your OS's native credential store — never in
  plain text, never in an environment variable, and never sent anywhere
  except directly to Google's Gemini API.
- You can view, replace, or remove it at any time via the **"Zmień klucz
  API"** button in the app's header.

### Run in development mode

```bash
npm run tauri dev
```

### Build a production installer

```bash
npm run tauri build
```

The installer will be generated under `src-tauri/target/release/bundle/`.

## Project structure
├── src/ # React frontend
│ ├── App.tsx # Main UI and state
│ ├── ThreeBackground.tsx # Animated 3D background
│ └── ...
├── src-tauri/ # Rust backend
│ └── src/
│ ├── commands.rs # Tauri commands exposed to the frontend
│ ├── market_engine.rs # Yahoo Finance data fetching
│ ├── analysis_engine.rs # Correlation & volatility calculations
│ ├── ai_engine.rs # Gemini API integration & Pine Script generation
│ ├── news_engine.rs # RSS fetching & filtering
│ ├── history_store.rs # Local snapshot persistence
│ ├── keychain.rs # Secure API key storage (OS credential store)
│ └── models.rs # Shared data structures
└── ...

## Security

- The Gemini API key never touches the frontend/JavaScript layer — it's
  written to and read from the OS credential store entirely within the Rust
  backend.
- Dependencies are audited automatically every week (`cargo audit`,
  `npm audit`) and on every push, via GitHub Actions.
- Commit history is continuously scanned for accidentally committed secrets
  (`gitleaks`).
- Auto-updates are cryptographically signed; the app verifies the signature
  before installing any update.

See [SECURITY.md](./SECURITY.md) for the full policy and how to report a
vulnerability.

## Disclaimer

This project is for educational and informational purposes only. Nothing it
generates constitutes financial advice or a recommendation to buy or sell
any asset. Market correlations shown here measure linear historical
relationships only and do not predict future price movement. Investment
decisions are made at your own risk.

## Changelog

See [CHANGELOG.md](./CHANGELOG.md) for release history.

## License

MIT — see [LICENSE](./LICENSE).