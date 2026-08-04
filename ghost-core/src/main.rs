mod pty;
mod terminal;

use anyhow::Result;
use pty::ShellPty;
use std::io::{self, Read, Write};
use std::thread;
use terminal::RawModeGuard;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Ghost initializing PTY passthrough...");

    let shell_pty = ShellPty::spawn()?;
    let _raw_guard = RawModeGuard::new()?;

    let mut pty_reader = shell_pty.pair.master.try_clone_reader()?;
    let mut pty_writer = shell_pty.pair.master.take_writer()?;

    // Thread 1: PTY Master output -> Host stdout (Instant screen rendering)
    let output_handle = thread::spawn(move || {
        let mut buffer = [0u8; 4096];
        let mut stdout = io::stdout();
        loop {
            match pty_reader.read(&mut buffer) {
                Ok(0) => break, // EOF: Shell exited
                Ok(n) => {
                    let _ = stdout.write_all(&buffer[..n]);
                    let _ = stdout.flush();
                }
                Err(_) => break,
            }
        }
    });

    // Thread 2: Host stdin bytes -> PTY Master input (Instant single-keypress pass)
    let input_handle = thread::spawn(move || {
        let mut buffer = [0u8; 4096];
        let mut stdin = io::stdin();
        loop {
            match stdin.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => {
                    let _ = pty_writer.write_all(&buffer[..n]);
                    let _ = pty_writer.flush();
                }
                Err(_) => break,
            }
        }
    });

    // Keep the main process alive until the child shell terminates
    let _ = output_handle.join();
    let _ = input_handle.join();

    Ok(())
}