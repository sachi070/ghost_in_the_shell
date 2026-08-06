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
use std::thread;
use std::time::Duration;
use terminal::RawModeGuard;

fn main() -> anyhow::Result<()> {
    let mut shell_pty = ShellPty::spawn()?;
    let raw_guard = RawModeGuard::new()?;

    let mut pty_reader = shell_pty.master.try_clone_reader()?;
    let mut pty_writer = shell_pty.master.take_writer()?;

    // Background runtime for async IPC calls
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let handle = rt.handle().clone();

    // Thread 1: PTY Master -> Host stdout + Interception
    let reader_handle = thread::spawn(move || {
        let mut buffer = [0u8; 8192];
        let stdout = io::stdout();
        let mut stdout_lock = stdout.lock();
        let mut parser = BoundaryParser::new();
        let mut ring_buffer = RollingBuffer::new(50);

        loop {
            match pty_reader.read(&mut buffer) {
                Ok(0) => break, // PTY EOF when shell exits
                Ok(n) => {
                    let bytes = &buffer[..n];
                    ring_buffer.push_bytes(bytes);

                    if let CommandStatus::Finished { exit_code } = parser.parse_bytes(bytes) {
                        if exit_code != 0 {
                            let context = ring_buffer.get_context();

                            let failure_msg = format!(
                                "\r\n\x1b[33m[Ghost Intercepted Failure: Exit Code {}]\x1b[0m\r\n",
                                exit_code
                            );
                            let _ = stdout_lock.write_all(failure_msg.as_bytes());

                            handle.spawn(async move {
                                if let Ok(resp) = ipc_client::send_diagnose_request(
                                    "last_command",
                                    exit_code,
                                    &context,
                                )
                                .await
                                {
                                    let diag_msg = format!(
                                        "\x1b[36m[Ghost Diagnosis]: {}\x1b[0m\r\n\x1b[32m[Suggested Fix]: {}\x1b[0m\r\n",
                                        resp.diagnosis, resp.suggested_fix
                                    );
                                    let stdout = io::stdout();
                                    let mut lock = stdout.lock();
                                    let _ = lock.write_all(diag_msg.as_bytes());
                                    let _ = lock.flush();
                                }
                            });
                        }
                    }

                    if stdout_lock.write_all(bytes).is_err() {
                        break;
                    }
                    let _ = stdout_lock.flush();
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
                // Filter out KeyRelease events on Windows (prevents double typing)
                if key.kind == KeyEventKind::Press || key.kind == KeyEventKind::Repeat {
                    match key.code {
                        KeyCode::Char(c) => {
                            if key.modifiers.contains(KeyModifiers::CONTROL) {
                                match c {
                                    'c' | 'C' => { let _ = pty_writer.write_all(&[3]); }  // Ctrl+C
                                    'd' | 'D' => { let _ = pty_writer.write_all(&[4]); }  // Ctrl+D
                                    'z' | 'Z' => { let _ = pty_writer.write_all(&[26]); } // Ctrl+Z
                                    'l' | 'L' => { let _ = pty_writer.write_all(&[12]); } // Ctrl+L
                                    _ => {}
                                }
                            } else {
                                let mut buf = [0u8; 4];
                                let s = c.encode_utf8(&mut buf);
                                let _ = pty_writer.write_all(s.as_bytes());
                            }
                        }
                        KeyCode::Enter => {
                            let _ = pty_writer.write_all(b"\r");
                        }
                        KeyCode::Backspace => {
                            let _ = pty_writer.write_all(&[8]);
                        }
                        KeyCode::Tab => {
                            let _ = pty_writer.write_all(b"\t");
                        }
                        KeyCode::Esc => {
                            let _ = pty_writer.write_all(&[27]);
                        }
                        KeyCode::Up => {
                            let _ = pty_writer.write_all(b"\x1b[A");
                        }
                        KeyCode::Down => {
                            let _ = pty_writer.write_all(b"\x1b[B");
                        }
                        KeyCode::Right => {
                            let _ = pty_writer.write_all(b"\x1b[C");
                        }
                        KeyCode::Left => {
                            let _ = pty_writer.write_all(b"\x1b[D");
                        }
                        _ => {}
                    }
                    let _ = pty_writer.flush();
                }
            }
        }
    }

    // Clean teardown sequence
    drop(pty_writer);           // Signal EOF to child PTY stdin
    let _ = reader_handle.join(); // Wait for reader thread to finish draining PTY stdout
    drop(raw_guard);            // Restore normal terminal modes
    let _ = io::stdout().flush();

    Ok(())
}