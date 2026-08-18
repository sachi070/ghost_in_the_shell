#  Ghost in the Shell

**Ghost in the Shell** is a dual-tier, AI-augmented terminal environment that transparently wraps shell sessions (Bash, CMD, Git Bash), intercepts non-zero exit codes, diagnoses root causes via Groq AI, logs CLI failure history in SQLite, and enables one-character auto-execution of AI fixes.

##  Key Features

* **Transparent PTY Wrapping**: Spawns sub-shells inside a native Pseudo-Terminal (PTY) with non-blocking keypress forwarding (`Ctrl+C`, `Ctrl+D`, arrow keys, UTF-8).
* **Automatic Error Interception**: Detects command completion boundaries and exit codes (`!= 0`) without disturbing normal stdout/stderr rendering.
* **Groq LLM Diagnostics**: Streams surrounding terminal context (last 50 lines) to a local daemon powered by Groq LLaMA models for instant root-cause analysis.
* **Interactive Auto-Fix (`f` / `fix`)**: Stage AI-suggested fixes with a single letter (`f` or `fix`) and execute them safely with `[y/N]` interactive confirmation.
* **Silent Shell Alias Layer**: Registers startup no-op aliases (`f=':'`, `y=':'`) to prevent `bash: command not found` errors during fix injection.
* **`ghost doctor` Audit Command**: Built-in CLI history browser that displays past failures, diagnoses, and fixes stored in a persistent SQLite database (`ghost_session.db`).

## 🏗️ System Architecture

```text
+-----------------------------------------------------------------------------------+
|                                  GHOST ARCHITECTURE                               |
+-----------------------------------------------------------------------------------+
|                                                                                   |
|  [ User Input ]  --->  [ ghost-core (Rust PTY Runner) ]                           |
|                             |                                                     |
|                             +---> Raw Mode Terminal Loop (crossterm)              |
|                             +---> Boundary Parser & ANSI State Machine            |
|                             +---> Shared Thread-Safe PTY Writer                   |
|                             |                                                     |
|                             v (HTTP IPC / JSON)                                   |
|                        [ ghost_daemon (Python FastAPI) ]                          |
|                             |                                                     |
|                             +---> Groq LLaMA Diagnostic Engine                    |
|                             +---> SQLite Storage (ghost_session.db)               |
|                                                                                   |
+-----------------------------------------------------------------------------------+
```

## 🛠️ Technical Stack

| Layer               | Component       | Technologies                                          |
| ------------------- | --------------- | ----------------------------------------------------- |
| **Terminal Runner** | `ghost-core`    | Rust, `portable-pty`, `crossterm`, `tokio`, `reqwest` |
| **Backend Daemon**  | `ghost_daemon`  | Python 3.12, FastAPI, Uvicorn, Groq API, SQLite       |
| **IPC Pipeline**    | Local REST/JSON | Async HTTP over `http://127.0.0.1:8000`               |
| **Stream Parser**   | Boundary Engine | Custom ANSI/OSC State Machine & FIFO Rolling Buffer   |

## 🛠️ Installation & Prerequisites

### Requirements

* **Rust** (1.75+ toolchain with `cargo`)
* **Python** (3.12+ with `uv` package manager)
* **Groq API Key**

## 🚀 Getting Started

### 1. Clone the Repository

```bash
git clone https://github.com/sachi070/ghost-in-the-shell.git
cd ghost-in-the-shell
```

### 2. Set Up Environment Variables

Create a `.env` file inside `ghost_daemon/`:

```env
GROQ_API_KEY=your_groq_api_key_here
GROQ_MODEL=llama-3.3-70b-versatile
```

### 3. Launch the Backend Daemon (`ghost_daemon`)

In your first terminal window:

```bash
cd ghost_daemon
uv run python main.py
```

*Daemon starts listening on `http://127.0.0.1:8000`.*

### 4. Run the Terminal Runner (`ghost-core`)

In your second terminal window:

```bash
cd ghost-core
cargo run
```

##  Usage Example

### 1. Trigger an Error

Run a command that fails (returns exit code `!= 0`):

```bash
$ cat missing_config.json
cat: missing_config.json: No such file or directory

[Ghost Intercepted Failure: Exit Code 1]
[Ghost Diagnosis]: The file 'missing_config.json' does not exist in the current directory.
[Suggested Fix]: touch missing_config.json
[Type 'fix' or 'f' to auto-execute this fix]
```

### 2. Request Fix Execution

Type `f` or `fix` and press **Enter**:

```bash
$ f

[Ghost]: Execute fix 'touch missing_config.json'? [y/N]:
```

### 3. Confirm Execution

Type `y` and press **Enter**:

```bash
$ y

[Ghost Executing Fix]: touch missing_config.json
```

### 4. View Audit History

To review recent CLI interceptions and fixes stored in SQLite:

```bash
$ ghost doctor

=== Ghost Doctor: Recent CLI Interception History ===
[2026-08-12 09:30:15] Command: cat missing_config.json
  Diagnosis: The file 'missing_config.json' does not exist in the current directory.
  Suggested Fix: touch missing_config.json
```

## 📁 Repository Structure

```text
ghost-in-the-shell/
├── ghost-core/               # Rust Terminal Runner (PTY, Parser, IPC Client)
│   ├── src/
│   │   ├── boundary.rs       # Stream parsing & command boundary detector
│   │   ├── buffer.rs         # Rolling 50-line context buffer & ANSI filter
│   │   ├── ipc_client.rs     # Async HTTP client for ghost_daemon
│   │   ├── main.rs           # Multi-threaded PTY reader/writer & input loop
│   │   ├── pty.rs            # portable-pty sub-shell spawner
│   │   └── terminal.rs       # Raw mode terminal guard (RAII)
│   └── Cargo.toml
│
└── ghost_daemon/             # Python FastAPI Backend & AI Engine
    ├── main.py               # FastAPI server endpoints (/diagnose, /history)
    ├── database.py           # SQLite persistence layer (ghost_session.db)
    ├── groq_service.py       # Groq LLM prompt builder & diagnostic engine
    └── pyproject.toml
```
