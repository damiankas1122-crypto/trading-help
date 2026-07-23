# Security Policy

## Supported Versions

Trading Help is under active development. Only the latest released version
(see [releases](https://github.com/damiankas1122-crypto/trading-help/releases))
receives security fixes. Users are strongly encouraged to keep auto-update
enabled so they always run the latest signed build.

| Version | Supported          |
| ------- | ------------------ |
| Latest  | :white_check_mark: |
| Older   | :x:                 |

## Reporting a Vulnerability

If you discover a security vulnerability in Trading Help, please **do not**
open a public GitHub issue. Instead, report it privately:

- Use GitHub's [private vulnerability reporting](https://github.com/damiankas1122-crypto/trading-help/security/advisories/new)
  feature on this repository, **or**
- Open a draft security advisory directly from the "Security" tab.

Please include:
- A description of the vulnerability and its potential impact
- Steps to reproduce (proof-of-concept if possible)
- Affected version/commit

You can expect an initial response within a few days. Confirmed
vulnerabilities will be fixed and shipped via the auto-update pipeline;
credit will be given in the release notes unless you prefer to remain
anonymous.

## Scope

Trading Help is a desktop application (Tauri 2 / Rust / React) that:
- Fetches public market data (Yahoo Finance, unofficial API) and RSS news
- Sends aggregated market data to Google's Gemini API to generate
  AI-written trading commentary and Pine Script snippets
- Stores snapshots locally on disk (no server-side component, no user
  accounts, no PII collection)

Trading Help is provided for informational purposes only and does **not**
constitute financial or investment advice.

## Known Design Notes (for security researchers)

- Releases are built and signed via GitHub Actions (`tauri-plugin-updater`,
  minisign). The signing private key never touches the repository or CI
  logs in plaintext.
- The `GEMINI_API_KEY` is currently supplied via environment variable and
  is never committed to the repository.
- This document is maintained alongside an internal security audit;
  ongoing hardening work is tracked in the project roadmap.
