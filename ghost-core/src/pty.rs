use anyhow::{Context, Result};
use portable_pty::{native_pty_system, CommandBuilder, PtyPair, PtySize};
use std::env;

pub struct ShellPty {
    pub pair: PtyPair,
    pub child: Box<dyn portable_pty::Child + Send + Sync>,
}

impl ShellPty {
    pub fn spawn() -> Result<Self> {
        let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
        let pty_system = native_pty_system();

        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("Failed to allocate PTY pair")?;

        // Windows / Bash default shell lookup
        let shell = env::var("SHELL").unwrap_or_else(|_| {
            if cfg!(windows) {
                "cmd.exe".to_string()
            } else {
                "/bin/bash".to_string()
            }
        });

        let mut cmd = CommandBuilder::new(&shell);
        cmd.env("TERM", "xterm-256color");
        cmd.env("GHOST_INSIDE", "1");

        let child = pair
            .slave
            .spawn_command(cmd)
            .context("Failed to spawn shell into PTY slave")?;

        Ok(Self { pair, child })
    }

    pub fn resize(&self, cols: u16, rows: u16) -> Result<()> {
        self.pair.master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        Ok(())
    }
}