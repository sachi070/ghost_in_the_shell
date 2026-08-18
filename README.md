# Ghost in the Shell

> **An AI-powered, self-healing terminal wrapper built in Rust and Python that intercepts failed shell commands at the PTY level, diagnoses root causes with low-latency LLMs, and stages safe, human-in-the-loop fixes with near-zero execution overhead.**

---

## Overview

Most developer AI tools live in a separate chat window, disconnected from where errors actually happen. **Ghost in the Shell** is invisible developer infrastructure — it sits directly on the terminal's keystroke path using native pseudoterminals (PTY).

When a command, compiler, or deployment script exits with a non-zero status code:

1. **Ghost intercepts stdout/stderr** and process boundaries using injected `OSC 1337` escape markers.
2. **Streams the terminal context** to a local Python daemon over IPC.
3. **Diagnoses the root cause** using low-latency inference (Groq / OpenRouter), with automatic offline fallback to a local **Ollama** model.
4. **Evaluates command execution safety** across a 3-tier risk matrix.
5. **Stages the fix inline**, letting you run it instantly by typing `f` or `fix`.

---

## Features

- **Multi-Shell Auto-Detection** — Native support for **Bash**, **Zsh**, and **PowerShell** (`pwsh.exe` / `powershell.exe`), with platform-specific boundary markers and silent alias injection.
- **Zero Hot-Path Overhead** — Written in **Rust** using `portable-pty` for async byte forwarding and transparent raw-mode terminal handling.
- **Hybrid Cloud + Offline AI Engine**
  - **Primary:** Groq (`llama-3.3-70b-versatile`) or OpenRouter.
  - **Fallback:** Local inference via Ollama (`qwen2.5-coder` / `llama3.2`) with automatic failover when disconnected.
- **3-Tier Execution Safety Engine**
  - **Safe** (`touch`, `mkdir`, `npm install`) — standard `[y/N]` confirmation.
  - **High-Risk / Destructive** (`rm -rf`, `git reset --hard`, `DROP TABLE`) — warning banner, requires typing `CONFIRM`.
  - **Critical Block** (`rm -rf /`, `mkfs`, `dd if=/dev/zero of=/dev/sda`) — hard execution block with explanation.
- **Workspace-Aware SQLite Memory** — Detects Git roots (`git rev-parse --show-toplevel`) to isolate diagnostic history per repository and surface recurring-error counts.
- **`ghost doctor` Diagnostic Suite**
  - `ghost doctor` — inspect recent command failures and their fixes.
  - `ghost doctor --search "<query>"` — full-text search across past terminal errors.
  - `ghost doctor --stats` — breakdown of top failing commands, workspace error distribution, and AI-engine invocation counts.
  - `ghost doctor --export markdown --out report.md` — generate diagnostic audit reports in Markdown or JSON.

### Roadmap
- **"Explain this error" mode** — an on-demand explanation of the failure printed straight to the terminal, kept separate from the auto-fix flow.
- Optional pairing with a companion CLI scaffolding tool for generating pre-documented project folder structures (e.g. FastAPI `src/app` layouts).

---

## System Architecture

```text
┌──────────────────────────────────────────────────────────┐
│                     User's Terminal                      │
└──────────────────────────────────────────────────────────┘
                         │ Raw keystrokes / ANSI streams
                         ▼
┌──────────────────────────────────────────────────────────┐
│             Ghost Binary (ghost-core, Rust)              │
│                                                          │
│  - Allocates system PTY (portable-pty)                   │
│  - Spawns target shell (Bash / Zsh / PowerShell)         │
│  - BoundaryParser: listens for OSC 1337 exit markers     │
│  - RollingBuffer: ring buffer of last 50 console lines   │
│  - Safety Engine: evaluates RiskLevel (Safe/Warn/Block)  │
│  - IPC Client: JSON payload dispatch over HTTP           │
└──────────────────────────────────────────────────────────┘
                         │ Local IPC (HTTP / Port 8000)
                         ▼
┌──────────────────────────────────────────────────────────┐
│           Ghost Daemon (ghost_daemon, Python)            │
│                                                          │
│  Fast-Path Regex Matcher (instant common fixes)          │
│    -> miss escalates to:                                 │
│  Workspace Resolver (Git root detection & context)       │
│    -> then:                                              │
│  Hybrid AI Inference Engine                              │
│    - Primary: Groq / OpenRouter                          │
│    - Fallback: Local Ollama (qwen2.5-coder)              │
│    -> then:                                              │
│  SQLite Session Storage (ghost_session.db)               │
│    - Interception log, recurring counts, analytics       │
└──────────────────────────────────────────────────────────┘
```

---

## Getting Started

### Prerequisites

- **Rust toolchain** (Cargo 1.80+)
- **Python 3.10+** (managed via `uv` or `venv`)
- *(Optional)* **Ollama**, installed locally for air-gapped / offline fallback:
  ```bash
  ollama pull qwen2.5-coder:latest
  ```

### 1. Start the background daemon (`ghost_daemon`)

```bash
cd ghost_daemon

# Install dependencies using uv (or pip)
uv sync

# Configure environment keys
cp .env.example .env
```

Edit `.env`:

```ini
GROQ_API_KEY="gsk_your_groq_api_key_here"

# Optional fallback configuration:
# OPENROUTER_API_KEY="sk-or-..."
# GHOST_LLM_MODEL="llama-3.3-70b-versatile"
# OLLAMA_HOST="http://127.0.0.1:11434"
# GHOST_LOCAL_MODEL="qwen2.5-coder:latest"
```

Start the daemon:

```bash
uv run python main.py
```

### 2. Launch the transparent PTY shell (`ghost-core`)

In a new terminal window:

```bash
cd ghost-core
cargo run --release
```

Ghost spawns your default active shell wrapped inside the PTY layer.

---

## Usage & Examples

### 1. Intercepting a failure and auto-fixing

```bash
$ cat non_existent_file.txt
cat: non_existent_file.txt: No such file or directory

[Ghost Intercepted Failure: Exit Code 1]
[Ghost Diagnosis]: Target file 'non_existent_file.txt' does not exist in the current directory.
[Suggested Fix]: touch non_existent_file.txt
[Type 'fix' or 'f' to auto-execute this fix]

$ f
[Ghost]: Execute fix 'touch non_existent_file.txt'? [y/N]: y
[Ghost Executing Fix]: touch non_existent_file.txt
```

### 2. High-risk safety confirmation

When a proposed fix contains state-altering flags (`rm -rf`, `git reset --hard`, `DROP TABLE`), Ghost requires explicit confirmation:

```bash
$ f
[Ghost Safety Warning]: Command contains destructive/state-altering flags!
[Ghost]: Execute fix 'git reset --hard HEAD~1'? Type 'CONFIRM' to run: CONFIRM
[Ghost Executing Fix]: git reset --hard HEAD~1
```

### 3. Querying `ghost doctor`

```bash
# View recent intercepted failures
$ ghost doctor

# Full-text search across past error diagnoses
$ ghost doctor --search "npm"

# View recurring error analytics and engine breakdowns
$ ghost doctor --stats

# Export audit logs to Markdown
$ ghost doctor --export markdown --out debug_report.md
```

---

## Repository Structure

```text
ghost-in-the-shell/
├── ghost-core/                     # Rust systems / PTY layer
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs                 # Raw mode loop, key handling, and IPC dispatch
│       ├── pty.rs                  # Cross-platform shell spawn & boundary hook injection
│       ├── boundary.rs             # OSC 1337 escape token parser & exit code detection
│       ├── safety.rs               # 3-tier risk engine (Safe, HighRisk, Critical)
│       ├── buffer.rs               # Circular ring buffer for terminal stdout context
│       ├── doctor.rs               # ghost doctor CLI formatter, stats & export engine
│       ├── ipc_client.rs           # Client bindings to Python daemon (reqwest)
│       └── terminal.rs             # Raw terminal guard & ANSI reset routines
│
├── ghost_daemon/                   # Python / FastAPI AI backend
│   ├── pyproject.toml              # Dependency manifest (uv)
│   ├── main.py                     # FastAPI server routes & lifespan management
│   ├── llm_client.py               # Hybrid LLM engine (Groq -> OpenRouter -> Ollama)
│   ├── db.py                       # SQLite schema, migrations, and workspace analytics
│   ├── models.py                   # Pydantic schemas for request/response validation
│   └── ipc_server.py               # Auxiliary IPC socket listener
│
├── shell-hooks/                    # Injected boundary scripts
│   ├── ghost.bash                  # Bash PROMPT_COMMAND hook
│   ├── ghost.zsh                   # Zsh precmd hook
│   └── ghost.ps1                   # PowerShell $Function:prompt hook
│
└── README.md
```

---

## Security & Design Philosophy

- **Non-destructive by default** — Ghost never executes commands autonomously; every suggestion requires explicit user confirmation.
- **Prompt-injection defense** — raw error streams are sanitized and passed through structured JSON schemas (`response_format={"type": "json_object"}`) before parsing, so malicious shell output can't escape its boundaries.
- **Air-gapped operation** — sensitive or disconnected environments can run Ghost entirely on local hardware via Ollama, with no terminal context ever sent over the network.

---

## License

Distributed under the MIT License. See `LICENSE` for details.
