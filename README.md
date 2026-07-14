# [Trading Help — AI Market Co-Pilot]

A cross-platform desktop app that analyzes NASDAQ, S&P 500, Gold, and Silver in
real time, correlates them with live financial news, and turns the results
into plain-language market commentary plus ready-to-use TradingView Pine
Script indicators.

Built with **Rust (Tauri)** on the backend and **React + TypeScript** on the
frontend. Actively used by a real client for daily market briefings.

## Download

Grab the latest Windows installer (no build required) from the
[Releases page](https://github.com/damiankas1122-crypto/trading-help/releases/latest).

## Features

- **Cross-market correlation** — computes lagged correlation between NASDAQ
  and S&P 500 returns, so you can see which index tends to lead the other.
- **Precious metals analysis** — tracks the Gold/Silver Ratio (GSR) over a
  30-day window, including its correlation and rate of change.
- **Live news integration** — pulls a financial RSS feed and filters it by
  instrument, so market commentary is grounded in what's actually being
  reported.
- **AI-generated briefings** — combines the numeric analysis with relevant
  news into a readable, per-instrument summary (via the Gemini API).
- **Session-to-session comparison** — every run is saved locally, so each new
  briefing highlights what changed since the last one (morning vs. afternoon
  vs. evening).
- **One-click TradingView scripts** — generates and explains Pine Script v6
  indicators for index correlation and the Gold/Silver Ratio, ready to paste
  into TradingView.

## Tech stack

| Layer      | Technology                                  |
| ---------- | -------------------------------------------- |
| Backend    | Rust, Tauri 2                                |
| Frontend   | React, TypeScript, Vite, Tailwind CSS         |
| Market data| Yahoo Finance API                             |
| AI         | Google Gemini API (`gemini-2.5-flash`)        |
| News       | RSS feed parsing                              |
| Storage    | Local JSON snapshot (no external database)    |

## Getting started

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (stable toolchain)
- [Node.js](https://nodejs.org/) 18+
- A [Gemini API key](https://aistudio.google.com/apikey) (free tier is enough
  for personal use)

### Setup

```bash
# Clone the repository
git clone https://github.com/<damiankas1122-crypto>/trading-help.git
cd trading-help

# Install frontend dependencies
npm install
```

### Environment variable

The app reads your Gemini API key from an environment variable at runtime —
it is never stored in the code or config files.

```bash
# Windows (PowerShell) — persists across sessions
setx GEMINI_API_KEY "your-api-key-here"

# macOS / Linux
export GEMINI_API_KEY="your-api-key-here"
```

> Restart your terminal (and IDE) after setting this so the new value is
> picked up.

### Run in development mode

```bash
npm run tauri dev
```

### Build a production installer

```bash
npm run tauri build
```

The installer will be generated under
`src-tauri/target/release/bundle/`.

## Project structure

```
├── src/                    # React frontend
│   ├── App.tsx              # Main UI and state
│   ├── ThreeBackground.tsx  # Animated 3D background
│   └── ...
├── src-tauri/               # Rust backend
│   └── src/
│       ├── commands.rs      # Tauri commands exposed to the frontend
│       ├── market_engine.rs # Yahoo Finance data fetching
│       ├── analysis_engine.rs # Correlation & volatility calculations
│       ├── ai_engine.rs     # Gemini API integration & Pine Script generation
│       ├── news_engine.rs   # RSS fetching & filtering
│       ├── history_store.rs # Local snapshot persistence
│       └── models.rs        # Shared data structures
└── ...
```

## Disclaimer

This project is for educational and informational purposes only. Nothing it
generates constitutes financial advice. Market correlations shown here
measure linear relationships only and do not predict future price movement.

## License

MIT — see [LICENSE](./LICENSE).

