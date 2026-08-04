# Ghost in the Shell

> **AI-Powered Self-Healing Terminal Agent**
> *A transparent Rust/Python PTY wrapper that catches command failures, diagnoses errors via an LLM, and offers interactive, non-destructive fixes or plain-language explanations.*

---

## System Architecture

```
┌─────────────────────────────────────────────────────────┐
│                     User's Terminal                      │
└───────────────────────┬───────────────────────────────────┘
                         │ keystrokes / ANSI output
                         ▼
┌─────────────────────────────────────────────────────────┐
│              Ghost Core (Rust Binary)                   │
│  - Master PTY allocation & raw-mode signal forwarding   │
│  - Intercepts shell hook boundary exit codes            │
│  - Maintains rolling stdout/stderr buffer               │
│  - Renders inline interactive patch UI                  │
└───────────────────────┬───────────────────────────────────┘
                         │ Unix Domain Socket (/tmp/ghost.sock)
                         ▼
┌─────────────────────────────────────────────────────────┐
│           Ghost Daemon (Python / FastAPI)                │
│  - Fast-path local regex rules engine (0-ms latency)    │
│  - Stack-aware context builder (cwd, package manifests) │
│  - Async LLM client (OpenRouter structured JSON)        │
│  - Async SQLite session audit log & recurring tracker   │
└─────────────────────────────────────────────────────────┘

```

---

## Core Features

* **Transparent PTY Passthrough:** Zero-overhead Rust binary sitting in the keystroke hot path (`portable-pty`).
* **Non-Destructive Safety Design:** Suggestions are rendered inline and strictly require explicit user approval (`[Enter]` to run, `[Esc]` to dismiss). Never auto-executes.
* **Dual Diagnostic Modes:** Supports both instant command patches and an `[e]` plain-language error explanation mode.
* **Local Fast-Path:** Resolves common CLI mistakes instantly via local regex patterns without an API network call.
* **Stack-Aware Context:** Automatically detects project roots (`package.json`, `Cargo.toml`, `requirements.txt`) to refine AI fixes.

---

## Tech Stack

* **Terminal Core:** Rust (`portable-pty`, `tokio`, `crossterm`, `nix`)
* **Backend Engine:** Python, FastAPI, `uv`, Pydantic
* **AI & Storage:** OpenRouter API, SQLite (`sqlmodel`, `aiosqlite`)
* **Hooks & Transport:** Unix Domain Sockets, Bash/Zsh shell hooks

---

## Quickstart

```bash
# Clone and install
git clone https://github.com/your-username/ghost-in-the-shell.git
cd ghost-in-the-shell
./scripts/install.sh

# Set your API key in ~/.ghost/config.toml
# openrouter_api_key = "your-key"

# Launch wrapper session
ghost

```
