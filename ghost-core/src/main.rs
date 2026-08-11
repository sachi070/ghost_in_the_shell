mod boundary;
mod buffer;
mod ipc_client;
mod pty;
mod terminal;

use boundary::{BoundaryParser, CommandStatus};
use buffer::RollingBuffer;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use pty::ShellPty;
use std::io::{self, Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use terminal::RawModeGuard;

fn main() -> anyhow::Result<()> {
    let mut shell_pty = ShellPty::spawn()?;
    let raw_guard = RawModeGuard::new()?;

    let pty_reader = shell_pty.master.try_clone_reader()?;
    let raw_writer = shell_pty.master.take_writer()?;

    // Thread-safe shared PTY writer (wrapped in Option so it can be cleanly dropped on exit)
    type SharedWriter = Arc<Mutex<Option<Box<dyn Write + Send>>>>;
    let shared_writer: SharedWriter = Arc::new(Mutex::new(Some(raw_writer)));
    let writer_for_reader = Arc::clone(&shared_writer);
    let writer_for_main = Arc::clone(&shared_writer);

    // Register no-op aliases ('f' and 'fix') in bash to suppress 'command not found'
    if let Ok(mut guard) = shared_writer.lock() {
        if let Some(ref mut writer) = *guard {
            let _ = writer.write_all(b"alias f=':' 2>/dev/null; alias fix=':' 2>/dev/null; clear\r");
            let _ = writer.flush();
        }
    }

    // Thread-safe storage for the latest AI suggested fix
    let last_suggested_fix: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let last_fix_reader = Arc::clone(&last_suggested_fix);

    // Multi-threaded runtime for async HTTP calls
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let handle = rt.handle().clone();

    // Thread 1: PTY Master -> Host stdout + Interception
    let mut reader = pty_reader;
    let reader_handle = thread::spawn(move || {
        let mut buffer = [0u8; 8192];
        let stdout = io::stdout();
        let mut stdout_lock = stdout.lock();
        let mut parser = BoundaryParser::new();
        let mut ring_buffer = RollingBuffer::new(50);

        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break, // PTY EOF when shell exits
                Ok(n) => {
                    let bytes = &buffer[..n];
                    ring_buffer.push_bytes(bytes);

                    // 1. Flush raw shell output to screen FIRST
                    if stdout_lock.write_all(bytes).is_err() {
                        break;
                    }
                    let _ = stdout_lock.flush();

                    // 2. Intercept command completions
                    if let CommandStatus::Finished { exit_code } = parser.parse_bytes(bytes) {
                        let last_cmd = ring_buffer.extract_last_command();
                        let trimmed_cmd = last_cmd.trim();

                        // A. Check if user typed 'fix' or 'f' to run the suggested fix
                        if trimmed_cmd == "fix" || trimmed_cmd == "f" {
                            if let Ok(mut fix_guard) = last_fix_reader.lock() {
                                if let Some(fix_cmd) = fix_guard.take() {
                                    let exec_msg = format!(
                                        "\r\n\x1b[32m[Ghost Executing Fix]: {}\x1b[0m\r\n",
                                        fix_cmd
                                    );
                                    let _ = stdout_lock.write_all(exec_msg.as_bytes());
                                    let _ = stdout_lock.flush();

                                    // Inject suggested fix command into PTY stdin
                                    if let Ok(mut writer_guard) = writer_for_reader.lock() {
                                        if let Some(ref mut writer) = *writer_guard {
                                            let _ = writer.write_all(fix_cmd.as_bytes());
                                            let _ = writer.write_all(b"\r");
                                            let _ = writer.flush();
                                        }
                                    }
                                } else {
                                    let _ = stdout_lock.write_all(
                                        b"\r\n\x1b[33m[Ghost]: No pending fix available.\x1b[0m\r\n",
                                    );
                                    let _ = stdout_lock.flush();
                                }
                            }
                        // B. Check if user ran 'ghost doctor'
                        } else if trimmed_cmd.contains("ghost doctor") {
                            let doctor_header = "\r\n\x1b[35m=== Ghost Doctor: Recent CLI Interception History ===\x1b[0m\r\n";
                            let _ = stdout_lock.write_all(doctor_header.as_bytes());

                            if let Ok(hist) = handle.block_on(ipc_client::fetch_history(5)) {
                                if hist.history.is_empty() {
                                    let _ = stdout_lock.write_all(
                                        b"No intercepted failures found in ghost_session.db.\r\n",
                                    );
                                } else {
                                    for record in hist.history {
                                        let entry = format!(
                                            "\x1b[33m[{}]\x1b[0m \x1b[1mCommand:\x1b[0m {}\r\n  \x1b[36mDiagnosis:\x1b[0m {}\r\n  \x1b[32mSuggested Fix:\x1b[0m {}\r\n\r\n",
                                            record.timestamp,
                                            record.command,
                                            record.diagnosis,
                                            record.suggested_fix
                                        );
                                        let _ = stdout_lock.write_all(entry.as_bytes());
                                    }
                                }
                            } else {
                                let _ = stdout_lock.write_all(
                                    b"\x1b[31mFailed to connect to ghost_daemon at http://127.0.0.1:8000\x1b[0m\r\n",
                                );
                            }
                            let _ = stdout_lock.flush();
                        // C. Standard non-zero exit code interception
                        } else if exit_code != 0 {
                            let context = ring_buffer.get_context();

                            let failure_msg = format!(
                                "\r\n\x1b[33m[Ghost Intercepted Failure: Exit Code {}]\x1b[0m\r\n",
                                exit_code
                            );
                            let _ = stdout_lock.write_all(failure_msg.as_bytes());

                            match handle.block_on(ipc_client::send_diagnose_request(
                                &last_cmd,
                                exit_code,
                                &context,
                            )) {
                                Ok(resp) => {
                                    // Store suggested fix in shared memory
                                    if let Ok(mut fix_guard) = last_fix_reader.lock() {
                                        *fix_guard = Some(resp.suggested_fix.clone());
                                    }

                                    let diag_msg = format!(
                                        "\x1b[36m[Ghost Diagnosis]: {}\x1b[0m\r\n\x1b[32m[Suggested Fix]: {}\x1b[0m\r\n\x1b[33m[Type 'fix' or 'f' to auto-execute this fix]\x1b[0m\r\n",
                                        resp.diagnosis, resp.suggested_fix
                                    );
                                    let _ = stdout_lock.write_all(diag_msg.as_bytes());
                                    let _ = stdout_lock.flush();
                                }
                                Err(e) => {
                                    let err_msg = format!(
                                        "\x1b[31m[Ghost Daemon Error]: Failed to reach diagnosis server ({})\x1b[0m\r\n",
                                        e
                                    );
                                    let _ = stdout_lock.write_all(err_msg.as_bytes());
                                    let _ = stdout_lock.flush();
                                }
                            }
                        }
                    }
                }
                Err(_) => break,
            }
        }
    });

    // Main Loop: Non-blocking stdin polling + Child Exit Monitoring
    loop {
        // 1. Check if child process (bash/cmd) has exited
        if let Ok(Some(_)) = shell_pty.child.try_wait() {
            break;
        }

        // 2. Non-blocking input polling (10ms tick rate)
        if event::poll(Duration::from_millis(10))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press || key.kind == KeyEventKind::Repeat {
                    if let Ok(mut writer_guard) = writer_for_main.lock() {
                        if let Some(ref mut writer) = *writer_guard {
                            match key.code {
                                KeyCode::Char(c) => {
                                    if key.modifiers.contains(KeyModifiers::CONTROL) {
                                        match c {
                                            'c' | 'C' => { let _ = writer.write_all(&[3]); }  // Ctrl+C
                                            'd' | 'D' => { let _ = writer.write_all(&[4]); }  // Ctrl+D
                                            'z' | 'Z' => { let _ = writer.write_all(&[26]); } // Ctrl+Z
                                            'l' | 'L' => { let _ = writer.write_all(&[12]); } // Ctrl+L
                                            _ => {}
                                        }
                                    } else {
                                        let mut buf = [0u8; 4];
                                        let s = c.encode_utf8(&mut buf);
                                        let _ = writer.write_all(s.as_bytes());
                                    }
                                }
                                KeyCode::Enter => {
                                    let _ = writer.write_all(b"\r");
                                }
                                KeyCode::Backspace => {
                                    let _ = writer.write_all(&[8]);
                                }
                                KeyCode::Tab => {
                                    let _ = writer.write_all(b"\t");
                                }
                                KeyCode::Esc => {
                                    let _ = writer.write_all(&[27]);
                                }
                                KeyCode::Up => {
                                    let _ = writer.write_all(b"\x1b[A");
                                }
                                KeyCode::Down => {
                                    let _ = writer.write_all(b"\x1b[B");
                                }
                                KeyCode::Right => {
                                    let _ = writer.write_all(b"\x1b[C");
                                }
                                KeyCode::Left => {
                                    let _ = writer.write_all(b"\x1b[D");
                                }
                                _ => {}
                            }
                            let _ = writer.flush();
                        }
                    }
                }
            }
        }
    }

    // Clean teardown sequence: Explicitly close PTY stdin so reader loop sees EOF
    if let Ok(mut guard) = shared_writer.lock() {
        *guard = None; // Drops the underlying PTY writer
    }

    let _ = reader_handle.join(); // Reader thread receives EOF and joins cleanly
    drop(raw_guard);              // Restore normal terminal modes
    let _ = io::stdout().flush();

    Ok(())
}