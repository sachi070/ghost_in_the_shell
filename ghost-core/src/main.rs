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
    // Spawn sub-shell inside PTY master/slave pair and set raw terminal mode
    let mut shell_pty = ShellPty::spawn()?;
    let raw_guard = RawModeGuard::new()?;

    let pty_reader = shell_pty.master.try_clone_reader()?;
    let raw_writer = shell_pty.master.take_writer()?;

    // Thread-safe shared PTY writer wrapped in Option for clean teardown
    type SharedWriter = Arc<Mutex<Option<Box<dyn Write + Send>>>>;
    let shared_writer: SharedWriter = Arc::new(Mutex::new(Some(raw_writer)));
    let writer_for_reader = Arc::clone(&shared_writer);
    let writer_for_main = Arc::clone(&shared_writer);

    // Register no-op aliases in bash so trigger keywords don't trigger 'command not found'
    if let Ok(mut guard) = shared_writer.lock() {
        if let Some(ref mut writer) = *guard {
            let _ = writer.write_all(
                b"alias f=':' fix=':' y=':' yes=':' Y=':' YES=':' n=':' no=':' N=':' NO=':' 2>/dev/null; clear\r",
            );
            let _ = writer.flush();
        }
    }

    // Shared storage for cached AI fixes and pending [y/N] prompts
    let last_suggested_fix: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let last_fix_reader = Arc::clone(&last_suggested_fix);

    let pending_confirmation: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let pending_conf_reader = Arc::clone(&pending_confirmation);

    // Multi-threaded Tokio runtime for async HTTP calls to ghost_daemon
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let handle = rt.handle().clone();

    // Reader thread: Reads PTY stdout and handles command interceptions
    let mut reader = pty_reader;
    let reader_handle = thread::spawn(move || {
        let mut buffer = [0u8; 8192];
        let stdout = io::stdout();
        let mut stdout_lock = stdout.lock();
        let mut parser = BoundaryParser::new();
        let mut ring_buffer = RollingBuffer::new(50);

        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break, // PTY EOF on shell exit
                Ok(n) => {
                    let bytes = &buffer[..n];
                    ring_buffer.push_bytes(bytes);

                    // Output shell bytes directly to terminal
                    if stdout_lock.write_all(bytes).is_err() {
                        break;
                    }
                    let _ = stdout_lock.flush();

                    // Check for completed command boundary
                    if let CommandStatus::Finished { exit_code } = parser.parse_bytes(bytes) {
                        let last_cmd = ring_buffer.extract_last_command();
                        let trimmed_cmd = last_cmd.trim().to_lowercase();

                        // 1. Handle user answer to [y/N] prompt
                        let mut was_pending = false;
                        if let Ok(mut pending_guard) = pending_conf_reader.lock() {
                            if let Some(staged_fix) = pending_guard.take() {
                                was_pending = true;

                                let clean_ans = trimmed_cmd
                                    .chars()
                                    .filter(|c| c.is_alphanumeric())
                                    .collect::<String>();

                                if clean_ans == "y" || clean_ans == "yes" || clean_ans.starts_with('y') {
                                    let exec_msg = format!(
                                        "\r\n\x1b[32m[Ghost Executing Fix]: {}\x1b[0m\r\n",
                                        staged_fix
                                    );
                                    let _ = stdout_lock.write_all(exec_msg.as_bytes());
                                    let _ = stdout_lock.flush();

                                    if let Ok(mut writer_guard) = writer_for_reader.lock() {
                                        if let Some(ref mut writer) = *writer_guard {
                                            let _ = writer.write_all(staged_fix.as_bytes());
                                            let _ = writer.write_all(b"\r");
                                            let _ = writer.flush();
                                        }
                                    }
                                } else {
                                    let cancel_msg = "\r\n\x1b[31m[Ghost]: Fix execution canceled.\x1b[0m\r\n";
                                    let _ = stdout_lock.write_all(cancel_msg.as_bytes());
                                    let _ = stdout_lock.flush();
                                }
                            }
                        }

                        if was_pending {
                            continue;
                        }

                        // 2. Handle 'f' or 'fix' trigger keyword
                        let clean_cmd = trimmed_cmd
                            .chars()
                            .filter(|c| c.is_alphanumeric())
                            .collect::<String>();

                        if clean_cmd == "fix" || clean_cmd == "f" {
                            if let Ok(mut fix_guard) = last_fix_reader.lock() {
                                if let Some(fix_cmd) = fix_guard.take() {
                                    if let Ok(mut pending_guard) = pending_conf_reader.lock() {
                                        *pending_guard = Some(fix_cmd.clone());
                                    }

                                    let prompt_msg = format!(
                                        "\r\n\x1b[33m[Ghost]: Execute fix '{}'? [y/N]: \x1b[0m",
                                        fix_cmd
                                    );
                                    let _ = stdout_lock.write_all(prompt_msg.as_bytes());
                                    let _ = stdout_lock.flush();
                                } else {
                                    let _ = stdout_lock.write_all(
                                        b"\r\n\x1b[33m[Ghost]: No pending fix available.\x1b[0m\r\n",
                                    );
                                    let _ = stdout_lock.flush();
                                }
                            }
                        // 3. Handle 'ghost doctor' audit command
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
                        // 4. Handle failed command (exit code != 0)
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

    // Main loop: Listen for host keypresses and forward to PTY stdin
    loop {
        if let Ok(Some(_)) = shell_pty.child.try_wait() {
            break;
        }

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
                                KeyCode::Enter => { let _ = writer.write_all(b"\r"); }
                                KeyCode::Backspace => { let _ = writer.write_all(&[8]); }
                                KeyCode::Tab => { let _ = writer.write_all(b"\t"); }
                                KeyCode::Esc => { let _ = writer.write_all(&[27]); }
                                KeyCode::Up => { let _ = writer.write_all(b"\x1b[A"); }
                                KeyCode::Down => { let _ = writer.write_all(b"\x1b[B"); }
                                KeyCode::Right => { let _ = writer.write_all(b"\x1b[C"); }
                                KeyCode::Left => { let _ = writer.write_all(b"\x1b[D"); }
                                _ => {}
                            }
                            let _ = writer.flush();
                        }
                    }
                }
            }
        }
    }

    // Teardown: Close PTY stdin so reader thread receives EOF and exits cleanly
    if let Ok(mut guard) = shared_writer.lock() {
        *guard = None;
    }

    let _ = reader_handle.join();
    drop(raw_guard);
    let _ = io::stdout().flush();

    Ok(())
}